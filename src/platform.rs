use bevy::prelude::*;

use crate::collision::{Collider, Kinematic};
use crate::game::{GameState, GameplaySet};
use crate::WINDOW_HEIGHT;

pub const PLATFORM_WIDTH: f32 = 120.0;
pub const PLATFORM_HEIGHT: f32 = 20.0;
const PLATFORM_SPEED: f32 = 400.0;
/// Fixed vertical center of the platform (it only moves horizontally).
pub const PLATFORM_Y: f32 = -WINDOW_HEIGHT / 2.0 + 40.0;

#[derive(Component)]
pub struct Platform;

pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_platform)
            .add_systems(Update, move_platform.in_set(GameplaySet::Movement));
    }
}

pub fn spawn_platform(mut commands: Commands) {
    commands.spawn((
        Platform,
        // Despawned automatically when we leave `Playing`, so restart cleans it up.
        DespawnOnExit(GameState::Playing),
        // Kinematic collider: the collision system keeps it out of the walls.
        Collider {
            half_size: Vec2::new(PLATFORM_WIDTH, PLATFORM_HEIGHT) / 2.0,
        },
        Kinematic,
        Sprite::from_color(
            Color::srgb(0.1, 0.45, 0.65),
            Vec2::new(PLATFORM_WIDTH, PLATFORM_HEIGHT),
        ),
        Transform::from_xyz(0.0, PLATFORM_Y, 0.0),
    ));
}

fn move_platform(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Platform>>,
) {
    let mut direction = 0.0;
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        direction -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        direction += 1.0;
    }

    // Move freely; the collision system stops the platform at the walls.
    for mut transform in &mut query {
        transform.translation.x += direction * PLATFORM_SPEED * time.delta_secs();
    }
}
