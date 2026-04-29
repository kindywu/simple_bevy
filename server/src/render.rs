use shared::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct SpriteReady;

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::default(), GlobalTransform::default()));
}

pub fn spawn_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    new_players: Query<(Entity, &PlayerId, &PlayerColor), (With<PlayerId>, Without<SpriteReady>)>,
) {
    for (entity, _player_id, color) in new_players.iter() {
        let mesh = Triangle2d::new(
            Vec2::new(0.0, 20.0),
            Vec2::new(-15.0, -20.0),
            Vec2::new(15.0, -20.0),
        );
        commands.entity(entity).insert((
            SpriteReady,
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(Color::srgb(color.r, color.g, color.b))),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
    }
}

pub fn spawn_bullet_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    new_bullets: Query<(Entity, &Bullet), Without<SpriteReady>>,
) {
    for (entity, bullet) in new_bullets.iter() {
        let mesh = Triangle2d::new(
            Vec2::new(0.0, 6.0),
            Vec2::new(-4.0, -6.0),
            Vec2::new(4.0, -6.0),
        );
        commands.entity(entity).insert((
            SpriteReady,
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(Color::srgb(bullet.r, bullet.g, bullet.b))),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
    }
}

pub fn apply_position(
    mut entities: Query<(&Position, &Direction, &mut Transform)>,
) {
    for (pos, dir, mut transform) in entities.iter_mut() {
        transform.translation = Vec3::new(pos.x, pos.y, 0.0);
        transform.rotation = Quat::from_rotation_z(dir.angle);
    }
}

pub fn apply_bullet_position(
    mut bullets: Query<(&Bullet, &mut Transform)>,
) {
    for (bullet, mut transform) in bullets.iter_mut() {
        transform.translation = Vec3::new(bullet.x, bullet.y, 0.0);
        transform.rotation = Quat::from_rotation_z(bullet.angle);
    }
}
