//! Agent-facing debug & scenario harness.
//!
//! Compiled only under the `debug` cargo feature. Produces machine-readable artifacts a
//! blind agent can inspect after a single run:
//!   - `debug_out/state.jsonl` — one JSON line per frame (platform x, held keys, edges, fps…)
//!   - `debug_out/frame_<n>.png` — screenshots at scenario-requested frames
//!
//! Set `BREK_SCENARIO=<path.json>` to drive a deterministic, bounded, scripted run.
//! Without it, the feature just logs state live and lets you press F12 to grab a screenshot.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Duration;

use bevy::diagnostic::{
    DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameCount, FrameTimeDiagnosticsPlugin,
};
use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::time::TimeUpdateStrategy;

use crate::ball::{Ball, BallState};
use crate::collision::Velocity;
use crate::game::GameState;
use crate::platform::{Platform, PLATFORM_WIDTH};
use crate::wall::Wall;
use crate::WINDOW_WIDTH;

/// Extra frames simulated after the run budget so async screenshots finish writing to disk.
const SCREENSHOT_FLUSH_FRAMES: u32 = 5;

// ---------------------------------------------------------------------------
// Scenario schema (deserialized from BREK_SCENARIO json)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct Scenario {
    #[serde(default)]
    #[allow(dead_code)] // human label only
    name: String,
    /// Exit after this many simulated frames.
    run_frames: u32,
    /// Don't log/screenshot until this frame (game still simulates from 0).
    #[serde(default)]
    start_frame: u32,
    #[serde(default)]
    init: Option<Init>,
    #[serde(default)]
    inputs: Vec<InputEvent>,
    #[serde(default)]
    screenshots: Vec<u32>,
}

#[derive(serde::Deserialize)]
struct Init {
    #[serde(default)]
    platform_x: Option<f32>,
}

#[derive(serde::Deserialize)]
struct InputEvent {
    at: u32,
    #[serde(default)]
    press: Vec<String>,
    #[serde(default)]
    release: Vec<String>,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct ActiveScenario(Option<Scenario>);

#[derive(Resource)]
struct DebugPaths {
    out_dir: PathBuf,
}

#[derive(Resource, Default)]
struct HeldKeys(HashSet<KeyCode>);

#[derive(Resource)]
struct StateLog {
    writer: BufWriter<File>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        let scenario = std::env::var("BREK_SCENARIO")
            .ok()
            .map(|p| load_scenario(&p));

        let out_dir =
            PathBuf::from(std::env::var("BREK_OUT").unwrap_or_else(|_| "debug_out".into()));
        std::fs::create_dir_all(&out_dir).expect("create debug output dir");
        let log_file = File::create(out_dir.join("state.jsonl")).expect("create state.jsonl");

        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(EntityCountDiagnosticsPlugin::default())
            .insert_resource(DebugPaths { out_dir })
            .insert_resource(HeldKeys::default())
            .insert_resource(StateLog {
                writer: BufWriter::new(log_file),
            })
            .add_systems(Update, screenshot_hotkey)
            .add_systems(PostUpdate, log_state);

        if scenario.is_some() {
            // Fixed timestep => "frame N" is reproducible across runs.
            app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1.0 / 60.0,
            )))
            .add_systems(
                OnEnter(GameState::Playing),
                apply_init.after(crate::platform::spawn_platform),
            )
            .add_systems(PreUpdate, inject_input.after(InputSystems))
            .add_systems(Update, take_scheduled_screenshots)
            .add_systems(PostUpdate, bounded_exit);
        }

        app.insert_resource(ActiveScenario(scenario));
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Apply scenario initial conditions once, after the platform is spawned.
fn apply_init(scenario: Res<ActiveScenario>, mut query: Query<&mut Transform, With<Platform>>) {
    let Some(scenario) = &scenario.0 else {
        return;
    };
    let Some(init) = &scenario.init else {
        return;
    };
    if let Some(x) = init.platform_x {
        for mut transform in &mut query {
            transform.translation.x = x;
        }
    }
}

/// Feed scripted keypresses into `ButtonInput` so `move_platform` reacts as if the user
/// were pressing keys. Runs after Bevy's real input system so our state wins.
fn inject_input(
    scenario: Res<ActiveScenario>,
    frames: Res<FrameCount>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut held: ResMut<HeldKeys>,
) {
    let Some(scenario) = &scenario.0 else {
        return;
    };
    let frame = frames.0;
    for event in &scenario.inputs {
        if event.at != frame {
            continue;
        }
        for name in &event.release {
            if let Some(key) = key_from_name(name) {
                held.0.remove(&key);
            }
        }
        for name in &event.press {
            if let Some(key) = key_from_name(name) {
                held.0.insert(key);
            }
        }
    }

    // Injected input is authoritative every frame: wipe any real/stray OS key state (the
    // window can steal focus when it pops up) and assert exactly the scripted held set, so
    // runs are deterministic regardless of what the real keyboard is doing.
    keyboard.release_all();
    for key in &held.0 {
        keyboard.press(*key);
    }
}

/// Capture a screenshot on any frame listed in the scenario.
fn take_scheduled_screenshots(
    scenario: Res<ActiveScenario>,
    frames: Res<FrameCount>,
    paths: Res<DebugPaths>,
    mut commands: Commands,
) {
    let Some(scenario) = &scenario.0 else {
        return;
    };
    let frame = frames.0;
    if scenario.screenshots.contains(&frame) {
        let path = paths.out_dir.join(format!("frame_{frame:04}.png"));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

/// Manual screenshot for live (non-scenario) debugging.
fn screenshot_hotkey(
    keyboard: Res<ButtonInput<KeyCode>>,
    frames: Res<FrameCount>,
    paths: Res<DebugPaths>,
    mut commands: Commands,
) {
    if keyboard.just_pressed(KeyCode::F12) {
        let frame = frames.0;
        let path = paths.out_dir.join(format!("manual_{frame:04}.png"));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

/// Append one JSON line describing this frame's state.
fn log_state(
    scenario: Res<ActiveScenario>,
    frames: Res<FrameCount>,
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    held: Res<HeldKeys>,
    platform: Query<&Transform, With<Platform>>,
    walls: Query<(&Transform, &Sprite, &Visibility), With<Wall>>,
    ball: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
    entities: Query<Entity>,
    mut log: ResMut<StateLog>,
) {
    let frame = frames.0;
    let start = scenario.0.as_ref().map(|s| s.start_frame).unwrap_or(0);
    if frame < start {
        return;
    }

    let platform_x = platform.single().map(|t| t.translation.x).ok();
    let max_x = WINDOW_WIDTH / 2.0 - PLATFORM_WIDTH / 2.0;
    let (at_left, at_right) = match platform_x {
        Some(x) => (x <= -max_x + 0.5, x >= max_x - 0.5),
        None => (false, false),
    };

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed());

    let mut held_names: Vec<String> = held.0.iter().filter_map(|k| name_from_key(*k)).collect();
    held_names.sort();

    // Wall geometry as inner-face bounds so placement can be verified numerically (walls are
    // off-screen + invisible, so screenshots can't show them).
    let walls: Vec<_> = walls
        .iter()
        .map(|(transform, sprite, visibility)| {
            let size = sprite.custom_size.unwrap_or(Vec2::ZERO);
            let center = transform.translation;
            serde_json::json!({
                "cx": center.x,
                "cy": center.y,
                "w": size.x,
                "h": size.y,
                "min_x": center.x - size.x / 2.0,
                "max_x": center.x + size.x / 2.0,
                "min_y": center.y - size.y / 2.0,
                "max_y": center.y + size.y / 2.0,
                "visible": matches!(visibility, Visibility::Visible),
            })
        })
        .collect();

    let ball = ball.single().ok().map(|(transform, velocity, state)| {
        serde_json::json!({
            "x": transform.translation.x,
            "y": transform.translation.y,
            "vx": velocity.0.x,
            "vy": velocity.0.y,
            "speed": velocity.0.length(),
            "state": match state {
                BallState::Stuck => "stuck",
                BallState::Free => "free",
            },
        })
    });

    let line = serde_json::json!({
        "frame": frame,
        "platform_x": platform_x,
        "held": held_names,
        "at_left_edge": at_left,
        "at_right_edge": at_right,
        "fps": fps,
        "entities": entities.iter().count(),
        "walls": walls,
        "ball": ball,
        "dt": time.delta_secs(),
    });
    let _ = writeln!(log.writer, "{line}");
}

/// Exit cleanly once the run budget (plus flush grace) is reached.
fn bounded_exit(
    scenario: Res<ActiveScenario>,
    frames: Res<FrameCount>,
    mut log: ResMut<StateLog>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(scenario) = &scenario.0 else {
        return;
    };
    if frames.0 >= scenario.run_frames + SCREENSHOT_FLUSH_FRAMES {
        let _ = log.writer.flush();
        exit.write(AppExit::Success);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_scenario(path: &str) -> Scenario {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read scenario `{path}`: {e}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("failed to parse scenario `{path}`: {e}"))
}

fn key_from_name(name: &str) -> Option<KeyCode> {
    match name {
        "Left" | "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "Right" | "ArrowRight" => Some(KeyCode::ArrowRight),
        "A" | "a" | "KeyA" => Some(KeyCode::KeyA),
        "D" | "d" | "KeyD" => Some(KeyCode::KeyD),
        "R" | "r" | "KeyR" => Some(KeyCode::KeyR),
        "Space" | "Spacebar" | " " => Some(KeyCode::Space),
        _ => None,
    }
}

fn name_from_key(key: KeyCode) -> Option<String> {
    match key {
        KeyCode::ArrowLeft => Some("Left".into()),
        KeyCode::ArrowRight => Some("Right".into()),
        KeyCode::KeyA => Some("A".into()),
        KeyCode::KeyD => Some("D".into()),
        KeyCode::KeyR => Some("R".into()),
        KeyCode::Space => Some("Space".into()),
        _ => None,
    }
}
