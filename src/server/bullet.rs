use crate::server::{
    RespawnTimer, BOUNDARY_MARGIN, KILL_SCORE, RESPAWN_DELAY_SECS, VISIBLE_HALF_HEIGHT,
    VISIBLE_HALF_WIDTH,
};
use crate::shared::*;
use bevy::prelude::*;
use bevy_replicon::prelude::*;

use super::combat::{point_in_triangle, triangle_vertices};

#[derive(Component, Deref, DerefMut)]
pub struct ShootCooldown(pub Timer);

#[derive(Component, Deref, DerefMut)]
pub struct BulletLifetime(pub Timer);

pub fn tick_cooldowns(time: Res<Time>, mut players: Query<&mut ShootCooldown>) {
    for mut cd in players.iter_mut() {
        cd.0.tick(time.delta());
    }
}

pub fn server_handle_shoot(
    mut commands: Commands,
    mut shoot_msgs: MessageReader<FromClient<ShootInput>>,
    mut players: Query<(
        Entity,
        &PlayerId,
        &Position,
        &Direction,
        &PlayerColor,
        &mut ShootCooldown,
    ), Without<Dead>>,
    bullets: Query<&BulletOwner>,
) {
    for FromClient { client_id, message: _ } in shoot_msgs.read() {
        let sender_id = client_id_to_u64(*client_id);

        let Some((_player_entity, _pid, pos, dir, color, mut cooldown)) = players
            .iter_mut()
            .find(|(_, pid, _, _, _, _)| pid.0 == sender_id)
        else {
            continue;
        };

        if cooldown.0.remaining_secs() > 0.0 {
            continue;
        }

        let bullet_count = bullets
            .iter()
            .filter(|owner| owner.0 == sender_id)
            .count();
        if bullet_count >= MAX_BULLETS_PER_PLAYER {
            continue;
        }

        cooldown.0.reset();

        let (sin_a, cos_a) = dir.angle.sin_cos();
        let tip_x = pos.x - 20.0 * sin_a;
        let tip_y = pos.y + 20.0 * cos_a;

        info!(
            "🔫 发射子弹: 玩家={} pos=({:.0},{:.0}) tip=({:.0},{:.0}) angle={:.2} color=({:.2},{:.2},{:.2})",
            sender_id, pos.x, pos.y, tip_x, tip_y, dir.angle, color.r, color.g, color.b
        );

        commands.spawn((
            Replicated,
            Bullet,
            BulletOwner(sender_id),
            Position { x: tip_x, y: tip_y },
            Direction {
                angle: dir.angle,
            },
            PlayerColor {
                r: color.r,
                g: color.g,
                b: color.b,
            },
            BulletLifetime(Timer::from_seconds(
                BULLET_LIFETIME_SECS,
                TimerMode::Once,
            )),
        ));
    }
}

pub fn move_bullets(
    mut commands: Commands,
    time: Res<Time>,
    mut bullets: Query<(Entity, &Direction, &mut Position), With<Bullet>>,
) {
    let count = bullets.iter().count();
    if count > 0 {
        debug!("子弹移动: {} 颗子弹活跃", count);
    }
    let dt = time.delta_secs();
    let min_x = -VISIBLE_HALF_WIDTH + BOUNDARY_MARGIN;
    let max_x = VISIBLE_HALF_WIDTH - BOUNDARY_MARGIN;
    let min_y = -VISIBLE_HALF_HEIGHT + BOUNDARY_MARGIN;
    let max_y = VISIBLE_HALF_HEIGHT - BOUNDARY_MARGIN;

    for (entity, dir, mut pos) in bullets.iter_mut() {
        let (sin_a, cos_a) = dir.angle.sin_cos();
        pos.x += -sin_a * BULLET_SPEED * dt;
        pos.y += cos_a * BULLET_SPEED * dt;

        if pos.x < min_x || pos.x > max_x || pos.y < min_y || pos.y > max_y {
            commands.entity(entity).despawn();
        }
    }
}

pub fn bullet_lifetime(
    mut commands: Commands,
    time: Res<Time>,
    mut bullets: Query<(Entity, &mut BulletLifetime)>,
) {
    for (entity, mut lifetime) in bullets.iter_mut() {
        lifetime.0.tick(time.delta());
        if lifetime.0.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn bullet_player_collision(
    mut commands: Commands,
    bullets: Query<(Entity, &Position, &BulletOwner)>,
    mut players: Query<(Entity, &PlayerId, &Position, &Direction, &mut Health), Without<Dead>>,
    mut score_query: Query<(&PlayerId, &mut Score)>,
) {
    for (bullet_entity, bullet_pos, bullet_owner) in bullets.iter() {
        for (player_entity, player_id, player_pos, player_dir, mut health) in players.iter_mut() {
            if player_id.0 == bullet_owner.0 {
                continue;
            }

            let (v0, v1, v2) = triangle_vertices(player_pos, player_dir);
            if point_in_triangle((bullet_pos.x, bullet_pos.y), v0, v1, v2) {
                commands.entity(bullet_entity).despawn();

                if health.0 > 0 {
                    health.0 -= 1;
                    info!(
                        "💥 玩家 {:?} 被击中! HP: {}/{}",
                        player_entity, health.0, MAX_HP
                    );
                }

                if health.0 == 0 {
                    commands.entity(player_entity).insert((
                        Dead,
                        RespawnTimer(Timer::from_seconds(
                            RESPAWN_DELAY_SECS,
                            TimerMode::Once,
                        )),
                    ));
                    info!("💀 玩家 {:?} 死亡", player_entity);

                    for (pid, mut score) in score_query.iter_mut() {
                        if pid.0 == bullet_owner.0 {
                            score.0 += KILL_SCORE;
                            info!("🏆 玩家 {:?} 得分: {} (总计: {})", pid.0, KILL_SCORE, score.0);
                            break;
                        }
                    }
                }

                break;
            }
        }
    }
}

fn client_id_to_u64(id: ClientId) -> u64 {
    match id {
        ClientId::Server => 0,
        ClientId::Client(entity) => entity.to_bits(),
    }
}
