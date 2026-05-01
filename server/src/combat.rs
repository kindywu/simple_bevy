use crate::{RespawnTimer, BOUNDARY_MARGIN, KILL_SCORE, MAX_SPAWN_ATTEMPTS, RESPAWN_DELAY_SECS, SAFE_SPAWN_DISTANCE, VISIBLE_HALF_HEIGHT, VISIBLE_HALF_WIDTH};
use shared::{Dead, Direction, Health, Position, Score, MAX_HP};
use bevy::prelude::*;
use rand::RngExt;
use std::collections::HashSet;

type Point2 = (f32, f32);

fn tip_world(pos: &Position, dir: &Direction) -> Point2 {
    let (sin_a, cos_a) = dir.angle.sin_cos();
    (pos.x - 20.0 * sin_a, pos.y + 20.0 * cos_a)
}

pub(crate) fn triangle_vertices(pos: &Position, dir: &Direction) -> (Point2, Point2, Point2) {
    let (sin_a, cos_a) = dir.angle.sin_cos();
    let px = pos.x;
    let py = pos.y;
    let tip = (px - 20.0 * sin_a, py + 20.0 * cos_a);
    let bl = (
        px - 15.0 * cos_a + 20.0 * sin_a,
        py - 15.0 * sin_a - 20.0 * cos_a,
    );
    let br = (
        px + 15.0 * cos_a + 20.0 * sin_a,
        py + 15.0 * sin_a - 20.0 * cos_a,
    );
    (tip, bl, br)
}

pub(crate) fn point_in_triangle(p: Point2, a: Point2, b: Point2, c: Point2) -> bool {
    let (px, py) = p;
    let (ax, ay) = a;
    let (bx, by) = b;
    let (cx, cy) = c;

    let v0x = cx - ax;
    let v0y = cy - ay;
    let v1x = bx - ax;
    let v1y = by - ay;
    let v2x = px - ax;
    let v2y = py - ay;

    let dot00 = v0x * v0x + v0y * v0y;
    let dot01 = v0x * v1x + v0y * v1y;
    let dot02 = v0x * v2x + v0y * v2y;
    let dot11 = v1x * v1x + v1y * v1y;
    let dot12 = v1x * v2x + v1y * v2y;

    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() < f32::EPSILON {
        return false;
    }
    let inv_denom = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    u >= 0.0 && v >= 0.0 && (u + v) <= 1.0
}

pub(crate) fn find_safe_spawn(alive_positions: &[Position]) -> Position {
    let min_x = -VISIBLE_HALF_WIDTH + BOUNDARY_MARGIN;
    let max_x = VISIBLE_HALF_WIDTH - BOUNDARY_MARGIN;
    let min_y = -VISIBLE_HALF_HEIGHT + BOUNDARY_MARGIN;
    let max_y = VISIBLE_HALF_HEIGHT - BOUNDARY_MARGIN;
    let mut rng = rand::rng();

    for _ in 0..MAX_SPAWN_ATTEMPTS {
        let x = rng.random_range(min_x..max_x);
        let y = rng.random_range(min_y..max_y);

        let safe = alive_positions.iter().all(|p| {
            let dx = x - p.x;
            let dy = y - p.y;
            (dx * dx + dy * dy).sqrt() >= SAFE_SPAWN_DISTANCE
        });

        if safe {
            return Position { x, y };
        }
    }

    Position {
        x: rng.random_range(min_x..max_x),
        y: rng.random_range(min_y..max_y),
    }
}

pub fn combat_detection(
    mut commands: Commands,
    players: Query<(Entity, &Position, &Direction), Without<Dead>>,
    mut score_query: Query<&mut Score>,
) {
    let entries: Vec<_> = players.iter().collect();
    if entries.len() < 2 {
        return;
    }

    let mut killed: HashSet<Entity> = HashSet::new();
    let mut score_deltas: Vec<(Entity, u32)> = Vec::new();

    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let (e_a, pos_a, dir_a) = entries[i];
            let (e_b, pos_b, dir_b) = entries[j];

            let a_killed = killed.contains(&e_a);
            let b_killed = killed.contains(&e_b);
            if a_killed && b_killed {
                continue;
            }

            let tip_a = tip_world(pos_a, dir_a);
            let (v0_b, v1_b, v2_b) = triangle_vertices(pos_b, dir_b);
            let tip_b = tip_world(pos_b, dir_b);
            let (v0_a, v1_a, v2_a) = triangle_vertices(pos_a, dir_a);

            let a_hits_b = point_in_triangle(tip_a, v0_b, v1_b, v2_b);
            let b_hits_a = point_in_triangle(tip_b, v0_a, v1_a, v2_a);

            match (a_hits_b, b_hits_a) {
                (true, true) => {
                    info!("💀 同归于尽: {:?} 和 {:?}", e_a, e_b);
                    killed.insert(e_a);
                    killed.insert(e_b);
                }
                (true, false) => {
                    if !a_killed && !b_killed {
                        info!("🔫 击杀: {:?} → {:?}", e_a, e_b);
                        score_deltas.push((e_a, KILL_SCORE));
                        killed.insert(e_b);
                    }
                }
                (false, true) => {
                    if !a_killed && !b_killed {
                        info!("🔫 击杀: {:?} → {:?}", e_b, e_a);
                        score_deltas.push((e_b, KILL_SCORE));
                        killed.insert(e_a);
                    }
                }
                (false, false) => {}
            }
        }
    }

    for entity in &killed {
        commands.entity(*entity).insert((
            Dead,
            RespawnTimer(Timer::from_seconds(RESPAWN_DELAY_SECS, TimerMode::Once)),
        ));
    }
    for (entity, delta) in &score_deltas {
        if let Ok(mut score) = score_query.get_mut(*entity) {
            score.0 += delta;
            info!("🏆 {:?} 得分: {} (总分: {})", entity, delta, score.0);
        }
    }
}

pub fn respawn_dead_players(
    time: Res<Time>,
    mut commands: Commands,
    mut dead_players: Query<(Entity, &mut RespawnTimer), With<Dead>>,
    alive_players: Query<&Position, Without<Dead>>,
) {
    for (entity, mut timer) in dead_players.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            let positions: Vec<Position> = alive_players.iter().copied().collect();
            let new_pos = find_safe_spawn(&positions);
            commands
                .entity(entity)
                .remove::<(Dead, RespawnTimer)>()
                .insert((new_pos, Health(MAX_HP)));
            info!("♻️ {:?} 重生在 ({:.0}, {:.0})", entity, new_pos.x, new_pos.y);
        }
    }
}
