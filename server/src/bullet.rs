use crate::{
    RespawnTimer, BOUNDARY_MARGIN, KILL_SCORE, RESPAWN_DELAY_SECS, VISIBLE_HALF_HEIGHT,
    VISIBLE_HALF_WIDTH, client_id_to_u64,
};
use shared::*;
use bevy::prelude::*;
use bevy_replicon::{prelude::*, shared::backend::connected_client::NetworkId};

use crate::combat::{point_in_triangle, triangle_vertices};

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
    clients: Query<&NetworkId>,
    bullets: Query<&Bullet>,
) {
    for FromClient { client_id, message: _ } in shoot_msgs.read() {
        let sender_id = client_id_to_u64(*client_id, &clients);

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
            .filter(|b| b.owner == sender_id)
            .count();
        if bullet_count >= MAX_BULLETS_PER_PLAYER {
            continue;
        }

        cooldown.0.reset();

        let (sin_a, cos_a) = dir.angle.sin_cos();
        let tip_x = pos.x - 20.0 * sin_a;
        let tip_y = pos.y + 20.0 * cos_a;

        info!(
            "🔫 发射子弹: owner={} pos=({:.0},{:.0}) angle={:.2} color=({:.2},{:.2},{:.2})",
            sender_id, tip_x, tip_y, dir.angle, color.r, color.g, color.b
        );

        commands.spawn((
            Replicated,
            Bullet {
                owner: sender_id,
                x: tip_x,
                y: tip_y,
                angle: dir.angle,
                speed: BULLET_SPEED,
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
    mut bullets: Query<(Entity, &mut Bullet)>,
) {
    let dt = time.delta_secs();
    let min_x = -VISIBLE_HALF_WIDTH + BOUNDARY_MARGIN;
    let max_x = VISIBLE_HALF_WIDTH - BOUNDARY_MARGIN;
    let min_y = -VISIBLE_HALF_HEIGHT + BOUNDARY_MARGIN;
    let max_y = VISIBLE_HALF_HEIGHT - BOUNDARY_MARGIN;

    for (entity, mut bullet) in bullets.iter_mut() {
        let (sin_a, cos_a) = bullet.angle.sin_cos();
        bullet.x += -sin_a * bullet.speed * dt;
        bullet.y += cos_a * bullet.speed * dt;

        if bullet.x < min_x || bullet.x > max_x || bullet.y < min_y || bullet.y > max_y {
            commands.entity(entity).despawn();
        }
    }
}

pub fn bullet_lifetime(
    mut commands: Commands,
    time: Res<Time>,
    mut bullets: Query<(Entity, &mut BulletLifetime), With<Bullet>>,
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
    bullets: Query<(Entity, &Bullet)>,
    mut players: Query<(Entity, &PlayerId, &Position, &Direction, &mut Health), Without<Dead>>,
    mut score_query: Query<(&PlayerId, &mut Score)>,
) {
    for (bullet_entity, bullet) in bullets.iter() {
        for (player_entity, player_id, player_pos, player_dir, mut health) in players.iter_mut() {
            if player_id.0 == bullet.owner {
                continue;
            }

            let (v0, v1, v2) = triangle_vertices(player_pos, player_dir);
            if point_in_triangle((bullet.x, bullet.y), v0, v1, v2) {
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
                        if pid.0 == bullet.owner {
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
