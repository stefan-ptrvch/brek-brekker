//! Simple AABB collision detection and resolution.
//!
//! Every collidable entity carries a [`Collider`] plus one body-type marker:
//! - [`Static`]   — never moves (walls).
//! - [`Kinematic`]— moved externally, e.g. by input (the platform); pushed out of statics, and
//!   a solid obstacle for dynamics, but never moved by them.
//! - [`Dynamic`]  — moved by its [`Velocity`] (the ball); pushed out of everything else and
//!   reflects its velocity on contact.
//!
//! Bevy's `bevy::math::bounding::Aabb2d` only offers a boolean intersection test, so we compute
//! the minimum-translation vector (MTV) ourselves.

use bevy::prelude::*;

use crate::game::GameplaySet;

/// Axis-aligned collision box, centered on the entity's `Transform`.
#[derive(Component)]
pub struct Collider {
    pub half_size: Vec2,
}

/// Immovable body (walls).
#[derive(Component)]
pub struct Static;

/// Externally-moved body (platform): pushed out of statics, never moved by dynamics.
#[derive(Component)]
pub struct Kinematic;

/// Velocity-driven body (ball): pushed out of everything and reflects on contact.
#[derive(Component)]
pub struct Dynamic;

/// Linear velocity in world units per second.
#[derive(Component)]
pub struct Velocity(pub Vec2);

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (resolve_kinematic, resolve_dynamic)
                .chain()
                .in_set(GameplaySet::Collision),
        );
    }
}

/// Minimum translation vector to push box A out of box B, or `None` if they don't overlap.
/// Resolves along the axis of least penetration.
fn aabb_mtv(a_center: Vec2, a_half: Vec2, b_center: Vec2, b_half: Vec2) -> Option<Vec2> {
    let delta = a_center - b_center;
    let overlap_x = (a_half.x + b_half.x) - delta.x.abs();
    let overlap_y = (a_half.y + b_half.y) - delta.y.abs();
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }
    if overlap_x < overlap_y {
        Some(Vec2::new(overlap_x * delta.x.signum(), 0.0))
    } else {
        Some(Vec2::new(0.0, overlap_y * delta.y.signum()))
    }
}

/// Push each kinematic body out of any static collider it overlaps (platform vs walls).
fn resolve_kinematic(
    mut kinematics: Query<(&mut Transform, &Collider), (With<Kinematic>, Without<Static>)>,
    statics: Query<(&Transform, &Collider), (With<Static>, Without<Kinematic>)>,
) {
    for (mut transform, collider) in &mut kinematics {
        for (static_transform, static_collider) in &statics {
            if let Some(mtv) = aabb_mtv(
                transform.translation.truncate(),
                collider.half_size,
                static_transform.translation.truncate(),
                static_collider.half_size,
            ) {
                transform.translation.x += mtv.x;
                transform.translation.y += mtv.y;
            }
        }
    }
}

/// Push each dynamic body out of every other collider and reflect its velocity (ball bounces).
fn resolve_dynamic(
    mut dynamics: Query<(&mut Transform, &Collider, &mut Velocity), With<Dynamic>>,
    obstacles: Query<(&Transform, &Collider), Without<Dynamic>>,
) {
    for (mut transform, collider, mut velocity) in &mut dynamics {
        for (obstacle_transform, obstacle_collider) in &obstacles {
            // Re-read the center each time so multiple contacts resolve against the
            // already-corrected position.
            if let Some(mtv) = aabb_mtv(
                transform.translation.truncate(),
                collider.half_size,
                obstacle_transform.translation.truncate(),
                obstacle_collider.half_size,
            ) {
                transform.translation.x += mtv.x;
                transform.translation.y += mtv.y;
                // Reflect (no energy loss) along the contact axis, only when moving into it.
                if mtv.x != 0.0 && velocity.0.x * mtv.x < 0.0 {
                    velocity.0.x = -velocity.0.x;
                }
                if mtv.y != 0.0 && velocity.0.y * mtv.y < 0.0 {
                    velocity.0.y = -velocity.0.y;
                }
            }
        }
    }
}
