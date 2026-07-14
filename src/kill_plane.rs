//! The kill plane: a full-width sensor along the bottom edge. When the ball touches it the
//! ball has been lost, so the game restarts.
//!
//! It carries a `Collider` but no body marker, so it's a pure sensor — the collision system
//! never bounces the ball off it (see `resolve_dynamic` in [`crate::collision`]); instead
//! `check_kill_plane` detects the overlap and triggers a restart.

use bevy::prelude::*;

use crate::ball::Ball;
use crate::collision::{aabb_overlap, Collider};
use crate::game::{GameState, GameplaySet};
use crate::{WINDOW_HEIGHT, WINDOW_WIDTH};

const KILL_PLANE_THICKNESS: f32 = 20.0;

#[derive(Component)]
pub struct KillPlane;

pub struct KillPlanePlugin;

impl Plugin for KillPlanePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_kill_plane)
            .add_systems(
                Update,
                check_kill_plane
                    .after(GameplaySet::Collision)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_kill_plane(mut commands: Commands) {
    let size = Vec2::new(WINDOW_WIDTH, KILL_PLANE_THICKNESS);
    // Top face flush with the screen bottom; body just outside (like the boundary walls).
    let center_y = -WINDOW_HEIGHT / 2.0 - KILL_PLANE_THICKNESS / 2.0;
    commands.spawn((
        KillPlane,
        DespawnOnExit(GameState::Playing),
        // Sensor: a collider with no body marker, so the ball falls into it rather than
        // bouncing off it.
        Collider {
            half_size: size / 2.0,
        },
        // Invisible in play; red so flipping visibility reveals it while debugging.
        Sprite::from_color(Color::srgb(1.0, 0.0, 0.0), size),
        Transform::from_xyz(0.0, center_y, 0.0),
        Visibility::Hidden,
    ));
}

/// Restart the game as soon as a ball touches the kill plane.
fn check_kill_plane(
    balls: Query<(&Transform, &Collider), With<Ball>>,
    plane: Query<(&Transform, &Collider), With<KillPlane>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((plane_transform, plane_collider)) = plane.single() else {
        return;
    };
    for (ball_transform, ball_collider) in &balls {
        if aabb_overlap(
            ball_transform.translation.truncate(),
            ball_collider.half_size,
            plane_transform.translation.truncate(),
            plane_collider.half_size,
        ) {
            next_state.set(GameState::Restarting);
            return;
        }
    }
}
