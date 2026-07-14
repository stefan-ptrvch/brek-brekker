//! Reusable wall entity plus the three invisible screen-boundary walls.
//!
//! A [`Wall`] is a rectangular boundary. Callers pick its position, dimensions, color and
//! visibility via [`WallSpec`], so the same entity works as an invisible collider or a
//! visible (e.g. debug) rectangle. Follows the restart convention: spawned in
//! `OnEnter(GameState::Playing)` and marked `DespawnOnExit(Playing)` (see [`crate::game`]).

use bevy::prelude::*;

use crate::game::GameState;
use crate::{WINDOW_HEIGHT, WINDOW_WIDTH};

/// Thickness of the screen-boundary walls: thin, but each sits fully outside the visible
/// area with its inner face flush to the screen edge.
const WALL_THICKNESS: f32 = 20.0;

#[derive(Component)]
pub struct Wall;

/// Creation parameters for a [`Wall`]. `size` is the full width/height; `position` is the
/// center; `color`/`visible` control appearance (invisible walls still collide).
pub struct WallSpec {
    pub position: Vec2,
    pub size: Vec2,
    pub color: Color,
    pub visible: bool,
}

impl Default for WallSpec {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            size: Vec2::ZERO,
            color: Color::WHITE,
            visible: true,
        }
    }
}

/// Spawn a wall from a [`WallSpec`], returning its entity.
pub fn spawn_wall(commands: &mut Commands, spec: WallSpec) -> Entity {
    let visibility = if spec.visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    commands
        .spawn((
            Wall,
            DespawnOnExit(GameState::Playing),
            Sprite::from_color(spec.color, spec.size),
            Transform::from_xyz(spec.position.x, spec.position.y, 0.0),
            visibility,
        ))
        .id()
}

pub struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_boundary_walls);
    }
}

/// The three invisible boundary walls (left, right, top). Each is just outside the visible
/// screen with its inner face on the screen edge. Vertical walls overrun the top row by
/// `WALL_THICKNESS` so the top corners are fully covered with no seam. No bottom wall — that
/// edge is guarded by the platform.
fn boundary_wall_specs() -> [WallSpec; 3] {
    let half_w = WINDOW_WIDTH / 2.0;
    let half_h = WINDOW_HEIGHT / 2.0;
    let t = WALL_THICKNESS;
    // Red so flipping `visible` to true reveals the walls while debugging.
    let debug_color = Color::srgb(1.0, 0.0, 0.0);

    [
        // Left: inner face at x = -half_w, body outside to the left.
        WallSpec {
            position: Vec2::new(-half_w - t / 2.0, 0.0),
            size: Vec2::new(t, WINDOW_HEIGHT + 2.0 * t),
            color: debug_color,
            visible: false,
        },
        // Right: inner face at x = +half_w, body outside to the right.
        WallSpec {
            position: Vec2::new(half_w + t / 2.0, 0.0),
            size: Vec2::new(t, WINDOW_HEIGHT + 2.0 * t),
            color: debug_color,
            visible: false,
        },
        // Top: inner face at y = +half_h, spans full width incl. corners.
        WallSpec {
            position: Vec2::new(0.0, half_h + t / 2.0),
            size: Vec2::new(WINDOW_WIDTH + 2.0 * t, t),
            color: debug_color,
            visible: false,
        },
    ]
}

fn spawn_boundary_walls(mut commands: Commands) {
    for spec in boundary_wall_specs() {
        spawn_wall(&mut commands, spec);
    }
}
