use bevy::prelude::*;

use crate::game::GameState;
use crate::{WINDOW_HEIGHT, WINDOW_WIDTH};

pub const PLATFORM_WIDTH: f32 = 120.0;
const PLATFORM_HEIGHT: f32 = 20.0;
const PLATFORM_SPEED: f32 = 400.0;

#[derive(Component)]
pub struct Platform;

pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_platform)
            .add_systems(
                Update,
                move_platform.run_if(in_state(GameState::Playing)),
            );
    }
}

pub fn spawn_platform(mut commands: Commands) {
    commands.spawn((
        Platform,
        // Despawned automatically when we leave `Playing`, so restart cleans it up.
        DespawnOnExit(GameState::Playing),
        Sprite::from_color(
            Color::srgb(0.1, 0.45, 0.65),
            Vec2::new(PLATFORM_WIDTH, PLATFORM_HEIGHT),
        ),
        Transform::from_xyz(0.0, -WINDOW_HEIGHT / 2.0 + 40.0, 0.0),
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

    let max_x = WINDOW_WIDTH / 2.0 - PLATFORM_WIDTH / 2.0;
    for mut transform in &mut query {
        transform.translation.x += direction * PLATFORM_SPEED * time.delta_secs();
        transform.translation.x = transform.translation.x.clamp(-max_x, max_x);
    }
}
