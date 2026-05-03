use crate::LocalClientId;
use shared::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct LocalSprite;

#[derive(Component)]
pub struct LocalPlayer;

pub fn spawn_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    local_id: Option<Res<LocalClientId>>,
    new_players: Query<(Entity, &PlayerId, &PlayerColor), (With<PlayerId>, Without<LocalSprite>)>,
) {
    for (entity, player_id, color) in new_players.iter() {
        let mesh = Triangle2d::new(
            Vec2::new(0.0, 20.0),
            Vec2::new(-15.0, -20.0),
            Vec2::new(15.0, -20.0),
        );
        let mut cmd = commands.entity(entity);
        cmd.insert((
            LocalSprite,
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(Color::srgb(color.r, color.g, color.b))),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
        if let Some(ref id) = local_id {
            if player_id.0 == id.0 {
                cmd.insert(LocalPlayer);
            }
        }
    }
}

pub fn spawn_bullet_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    new_bullets: Query<(Entity, &Bullet), Without<LocalSprite>>,
) {
    for (entity, bullet) in new_bullets.iter() {
        let mesh = Triangle2d::new(
            Vec2::new(0.0, 6.0),
            Vec2::new(-4.0, -6.0),
            Vec2::new(4.0, -6.0),
        );
        commands.entity(entity).insert((
            LocalSprite,
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(Color::srgb(bullet.r, bullet.g, bullet.b))),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
    }
}
