//! Simple AABB collision detection and resolution.
//!
//! Every collidable entity carries a [`Collider`]. Bodies marked [`Dynamic`] are pushed out
//! of static colliders (those without `Dynamic`) so they can't pass through them — that's how
//! the platform is stopped by the boundary walls. Bevy's `bevy::math::bounding::Aabb2d` only
//! offers a boolean intersection test, so we compute the minimum-translation vector ourselves.

use bevy::prelude::*;

use crate::game::GameplaySet;

/// Axis-aligned collision box, centered on the entity's `Transform`.
#[derive(Component)]
pub struct Collider {
    pub half_size: Vec2,
}

/// Marks a body that gets pushed out of static colliders on overlap.
#[derive(Component)]
pub struct Dynamic;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, resolve_collisions.in_set(GameplaySet::Collision));
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

/// Push every dynamic body out of any static collider it overlaps.
fn resolve_collisions(
    mut dynamics: Query<(&mut Transform, &Collider), With<Dynamic>>,
    statics: Query<(&Transform, &Collider), Without<Dynamic>>,
) {
    for (mut transform, dynamic) in &mut dynamics {
        for (static_transform, static_collider) in &statics {
            // Re-read the center each time so multiple contacts resolve against the
            // already-corrected position.
            if let Some(mtv) = aabb_mtv(
                transform.translation.truncate(),
                dynamic.half_size,
                static_transform.translation.truncate(),
                static_collider.half_size,
            ) {
                transform.translation.x += mtv.x;
                transform.translation.y += mtv.y;
            }
        }
    }
}
