//! Core game state and the restart mechanism.
//!
//! Restart is built on Bevy states so it scales as entities are added: every gameplay
//! entity is spawned in `OnEnter(GameState::Playing)` and marked `DespawnOnExit(Playing)`,
//! so Bevy despawns it automatically when we leave `Playing`. Pressing `R` bounces through
//! the transient `Restarting` state (Playing -> Restarting -> Playing), which despawns
//! everything on exit and re-runs all the `OnEnter(Playing)` spawners — no central restart
//! function to keep in sync.
//!
//! Adding a new entity later: spawn it in an `OnEnter(GameState::Playing)` system and give
//! it `DespawnOnExit(GameState::Playing)`. Gameplay resources that mutate during play should
//! be reset in an `OnEnter(GameState::Playing)` system too.

use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Normal gameplay.
    #[default]
    Playing,
    /// One-frame transient: entities have been despawned; immediately returns to `Playing`,
    /// which re-runs the spawners. Exists so a restart is a real state transition (you can't
    /// re-enter the state you're already in).
    Restarting,
}

/// Ordering of per-frame gameplay work. Chained so collisions resolve after all movement, and
/// gated to `Playing` at the set level (systems in these sets inherit the condition).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameplaySet {
    Movement,
    Collision,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .configure_sets(
                Update,
                (GameplaySet::Movement, GameplaySet::Collision)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                restart_on_key.run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Restarting), finish_restart);
    }
}

/// Leave `Playing` when `R` is pressed, which triggers the despawn-and-respawn cycle.
fn restart_on_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        next_state.set(GameState::Restarting);
    }
}

/// Immediately return to `Playing`, re-running every `OnEnter(Playing)` spawner.
fn finish_restart(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}
