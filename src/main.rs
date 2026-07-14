use bevy::prelude::*;
use bevy::window::PresentMode;

mod ball;
mod collision;
mod game;
mod platform;
mod wall;

use ball::BallPlugin;
use collision::CollisionPlugin;
use game::GamePlugin;
use platform::PlatformPlugin;
use wall::WallPlugin;

#[cfg(feature = "debug")]
mod debug;

pub const WINDOW_WIDTH: f32 = 800.0;
pub const WINDOW_HEIGHT: f32 = 600.0;

// In debug runs we uncap the framerate so bounded scenario runs finish fast (and the
// laptop stays cool); normal play keeps vsync on.
#[cfg(feature = "debug")]
const PRESENT_MODE: PresentMode = PresentMode::AutoNoVsync;
#[cfg(not(feature = "debug"))]
const PRESENT_MODE: PresentMode = PresentMode::AutoVsync;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32).into(),
            present_mode: PRESENT_MODE,
            resizable: false,
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.78, 0.72, 0.58)))
    .add_plugins(GamePlugin)
    .add_plugins(CollisionPlugin)
    .add_plugins(PlatformPlugin)
    .add_plugins(WallPlugin)
    .add_plugins(BallPlugin)
    .add_systems(Startup, setup);

    #[cfg(feature = "debug")]
    app.add_plugins(debug::DebugPlugin);

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
