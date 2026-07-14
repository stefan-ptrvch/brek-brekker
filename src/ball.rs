//! The ball: starts stuck to the platform, launches straight up on Space, then flies with no
//! gravity and bounces (via the collision system) off the platform and walls with no losses.

use bevy::prelude::*;

use crate::collision::{Collider, Dynamic, Velocity};
use crate::game::{GameState, GameplaySet};
use crate::platform::{Platform, PLATFORM_HEIGHT, PLATFORM_WIDTH, PLATFORM_Y};

// --- Settings ---
/// Radius ≈ an eighth of the platform width.
const BALL_RADIUS: f32 = PLATFORM_WIDTH / 8.0;
/// Constant launch/flight speed (world units per second).
const BALL_SPEED: f32 = 400.0;
/// Blood orange.
const BALL_COLOR: Color = Color::srgb(0.82, 0.25, 0.05);
/// Resting center height: sitting on top of the platform.
const BALL_STUCK_Y: f32 = PLATFORM_Y + PLATFORM_HEIGHT / 2.0 + BALL_RADIUS;

#[derive(Component)]
pub struct Ball;

/// Whether the ball is riding the platform or flying freely.
#[derive(Component, PartialEq)]
pub enum BallState {
    Stuck,
    Free,
}

/// Shared mesh + material handles, created once so restarts don't leak assets.
#[derive(Resource)]
struct BallAssets {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

pub struct BallPlugin;

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_ball)
            .add_systems(
                Update,
                (
                    launch_ball.before(GameplaySet::Movement),
                    move_ball.in_set(GameplaySet::Movement),
                    stick_ball.after(GameplaySet::Collision),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_ball(
    mut commands: Commands,
    cached: Option<Res<BallAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Create the mesh/material once and cache the handles so restarts reuse them (no asset
    // leak). Done here rather than at Startup because the initial `OnEnter(Playing)` runs
    // before `Startup`.
    let assets = match cached {
        Some(assets) => BallAssets {
            mesh: assets.mesh.clone(),
            material: assets.material.clone(),
        },
        None => {
            let assets = BallAssets {
                mesh: meshes.add(Circle::new(BALL_RADIUS)),
                material: materials.add(BALL_COLOR),
            };
            commands.insert_resource(BallAssets {
                mesh: assets.mesh.clone(),
                material: assets.material.clone(),
            });
            assets
        }
    };
    commands.spawn((
        Ball,
        BallState::Stuck,
        DespawnOnExit(GameState::Playing),
        // Always a dynamic body; while stuck its velocity is zero and it rests exactly on the
        // platform top (zero overlap), so collision resolution is a no-op until launch.
        Dynamic,
        Velocity(Vec2::ZERO),
        Collider {
            half_size: Vec2::splat(BALL_RADIUS),
        },
        Mesh2d(assets.mesh.clone()),
        MeshMaterial2d(assets.material.clone()),
        // z = 1 so the ball draws over the platform.
        Transform::from_xyz(0.0, BALL_STUCK_Y, 1.0),
    ));
}

/// Keep a stuck ball centered on top of the platform as it moves.
fn stick_ball(
    mut balls: Query<(&mut Transform, &BallState), With<Ball>>,
    platform: Query<&Transform, (With<Platform>, Without<Ball>)>,
) {
    let Ok(platform) = platform.single() else {
        return;
    };
    for (mut transform, state) in &mut balls {
        if *state == BallState::Stuck {
            transform.translation.x = platform.translation.x;
            transform.translation.y = BALL_STUCK_Y;
        }
    }
}

/// Space launches a stuck ball straight up at constant speed.
fn launch_ball(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut balls: Query<(&mut BallState, &mut Velocity), With<Ball>>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    for (mut state, mut velocity) in &mut balls {
        if *state == BallState::Stuck {
            *state = BallState::Free;
            velocity.0 = Vec2::new(0.0, BALL_SPEED);
        }
    }
}

/// Integrate a free ball's position from its velocity.
fn move_ball(time: Res<Time>, mut balls: Query<(&mut Transform, &Velocity, &BallState), With<Ball>>) {
    for (mut transform, velocity, state) in &mut balls {
        if *state == BallState::Free {
            transform.translation.x += velocity.0.x * time.delta_secs();
            transform.translation.y += velocity.0.y * time.delta_secs();
        }
    }
}
