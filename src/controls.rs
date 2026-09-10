//! Application-owned controls and comparison sequences hosted by PocketJS.
use super::*;
use pocket_ui_wgpu::UiOverlay;
use pocket3d::app::CursorMode;
use serde_json::{Value, json};

pub(super) struct Controls {
    pub open: bool,
    pub paused: bool,
    pub block_gameplay_frame: bool,
    pub size: (u32, u32),
    pub scale: f32,
    pub overlay: Option<UiOverlay>,
    runner: Option<Comparison>,
    progress: f32,
    outcome: &'static str,
    result: String,
    commands: Vec<String>,
    error: Option<String>,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            open: false,
            paused: false,
            block_gameplay_frame: false,
            size: (960, 600),
            scale: 1.0,
            overlay: None,
            runner: None,
            progress: 0.0,
            outcome: "idle",
            result: String::new(),
            commands: Vec::new(),
            error: None,
        }
    }
}
struct Comparison {
    scenario: String,
    turn: u64,
    ticks: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ControlReceipt {
    pub visible: bool,
    pub cursor_mode: &'static str,
    pub paused: bool,
    pub running: bool,
    pub progress: f32,
    pub outcome: &'static str,
    pub result: String,
    pub commands: Vec<String>,
    pub error: Option<String>,
}

pub fn shelter_scenario_ticks(name: &str) -> Option<u64> {
    Some(match name {
        "shelter-rain" => 480,
        "shelter-rain-moved" => 900,
        "shelter-screen" => 1200,
        "shelter-screen-water" => 1290,
        "shelter-dry" => 2400,
        "shelter-screen-open-water" => 2490,
        "shelter-firebreak" | "shelter-firebreak-dry" => 1800,
        _ => return None,
    })
}

/// The same action schedule drives pointer-started comparisons and headless
/// regressions. Recipes, action timing and acceptance stay in the application.
pub fn shelter_script_actions(name: &str, turn: u64) -> [bool; 4] {
    let Some(trial) = ShelterTrial::from_scenario(name) else {
        return [false; 4];
    };
    [
        turn == 2 && trial == ShelterTrial::Rain,
        turn == match trial {
            ShelterTrial::Rain => 420,
            ShelterTrial::Firebreak => 150,
            ShelterTrial::Screen => 2,
        },
        (name == "shelter-rain-moved" && turn == 480)
            || ((name == "shelter-dry" || name == "shelter-screen-open-water") && turn == 1200),
        (name == "shelter-screen-water" && turn == 1200)
            || (name == "shelter-screen-open-water" && turn == 2400)
            || (name == "shelter-firebreak" && turn == 2),
    ]
}

impl WorldGame {
    pub fn open_controls(&mut self) -> Result<()> {
        self.controls.open = true;
        self.controls.block_gameplay_frame = true;
        self.pending = PendingActions::default();
        self.ensure_overlay()?;
        self.update_controls(&Input::default());
        Ok(())
    }

    fn ensure_overlay(&mut self) -> Result<()> {
        if self.controls.overlay.is_none() {
            self.controls.overlay = Some(UiOverlay::new(
                include_str!(concat!(env!("OUT_DIR"), "/ui/main.js")),
                include_bytes!(concat!(env!("OUT_DIR"), "/ui/main.pak")),
                self.controls.size,
                self.controls.scale,
                2,
            )?);
        }
        Ok(())
    }

    /// Refresh display data after camera projection and the simulation tick.
    /// Only controls_frame routes live input; the passive HUD cannot take it.
    pub(super) fn refresh_presentation_ui(&mut self) {
        if let Err(error) = self.ensure_overlay() {
            self.controls.error = Some(error.to_string());
            return;
        }
        self.update_controls(&Input::default());
    }

    pub fn control_receipt(&self) -> ControlReceipt {
        let c = &self.controls;
        ControlReceipt {
            visible: c.open,
            cursor_mode: if c.open { "pointer" } else { "captured" },
            paused: c.paused,
            running: c.runner.is_some(),
            progress: c.progress,
            outcome: c.outcome,
            result: c.result.clone(),
            commands: c.commands.clone(),
            error: c.error.clone(),
        }
    }

    pub fn control_bounds(&self, name: &str) -> Option<(f32, f32, f32, f32)> {
        self.controls.overlay.as_ref()?.control_bounds(name)
    }

    pub(super) fn controls_frame(&mut self, input: &Input) -> bool {
        self.controls.block_gameplay_frame = false;
        if input.key_pressed(KeyCode::Tab) || input.key_pressed(KeyCode::Escape) {
            if self.controls.open {
                self.close_controls();
            } else if let Err(error) = self.open_controls() {
                self.controls.error = Some(error.to_string());
            }
            self.controls.block_gameplay_frame = true;
            return true;
        }
        if self.controls.open {
            self.controls.block_gameplay_frame = true;
            self.update_controls(input);
            return true;
        }
        false
    }

    fn close_controls(&mut self) {
        self.controls.paused = false;
        if self.controls.runner.take().is_some() {
            self.controls.outcome = "idle";
            self.controls.result = "Comparison stopped. Free play continues.".into();
        }
        if let Some(ui) = &self.controls.overlay {
            ui.cancel_pointer();
        }
        self.controls.open = false;
        self.controls.block_gameplay_frame = true;
        self.pending = PendingActions::default();
    }

    fn update_controls(&mut self, input: &Input) {
        let model = self.control_model();
        if let Some(ui) = self.controls.overlay.as_mut() {
            match ui.frame(input, &model) {
                Ok(commands) => {
                    for command in commands {
                        if !self.controls.open {
                            break;
                        }
                        self.control_command(command);
                    }
                }
                Err(error) => {
                    self.controls.error = Some(error.to_string());
                    self.controls.outcome = "failed";
                    self.controls.result = "UI error; see terminal output.".into();
                }
            }
        }
    }

    fn control_model(&self) -> Value {
        let receipt = self.shelter_receipt();
        let (trial, rain, screen_open, left, right) = if let Some(r) = &receipt {
            let (a, b) = if r.trial == ShelterTrial::Firebreak {
                (2, 7)
            } else {
                (0, 1)
            };
            let sample = |index: usize| {
                let s = &r.samples[index];
                let status = if r.trial == ShelterTrial::Firebreak {
                    if r.samples[index + 2].ever_ignited {
                        "Fire reached the end"
                    } else {
                        "Far end unburned"
                    }
                } else {
                    match s.ignition_status {
                        pocket3d_world::IgnitionStatus::Burning => "Burning",
                        pocket3d_world::IgnitionStatus::TooWet => "Too wet to ignite",
                        pocket3d_world::IgnitionStatus::TooCold => "Needs more heat",
                        pocket3d_world::IgnitionStatus::Exhausted => "Fuel exhausted",
                        _ => "Ready",
                    }
                };
                json!({"wet":s.moisture*100.0,"temperature":s.temperature_c,"status":status})
            };
            (
                match r.trial {
                    ShelterTrial::Rain => "rain",
                    ShelterTrial::Screen => "screen",
                    ShelterTrial::Firebreak => "firebreak",
                },
                r.rain,
                r.screen_open,
                sample(a),
                sample(b),
            )
        } else {
            (
                "orchard",
                false,
                false,
                json!({"wet":0,"temperature":24,"status":""}),
                json!({"wet":0,"temperature":24,"status":""}),
            )
        };
        let activity = if let Some(run) = &self.controls.runner {
            format!(
                "{}s / {}s - {}",
                run.turn / 60,
                run.ticks / 60,
                if self.controls.paused {
                    "Paused"
                } else if run.turn < 150 {
                    "Preparing the comparison"
                } else {
                    "Watch the world react"
                }
            )
        } else {
            "Click a comparison or try the controls below.".to_owned()
        };
        json!({"open":self.controls.open,"hud":if self.controls.open { Value::Null } else { self.hud_model() },
            "trial":trial,"rain":rain,"screenOpen":screen_open,"left":left,"right":right,
            "paused":self.controls.paused,"running":self.controls.runner.is_some(),"progress":self.controls.progress,
            "elapsed":self.world.tick() as f32/60.0,"activity":activity,"outcome":self.controls.outcome,"result":self.controls.result,
            "width":self.controls.size.0 as f32/self.controls.scale,"height":self.controls.size.1 as f32/self.controls.scale})
    }

    fn control_command(&mut self, command: Value) {
        let Some(action) = command["action"].as_str() else {
            return;
        };
        let value = command["value"].as_str().unwrap_or_default();
        println!("pocket-openworld: UI {action} {value}");
        self.controls.commands.push(if value.is_empty() {
            action.to_owned()
        } else {
            format!("{action}:{value}")
        });
        if self.controls.commands.len() > 64 {
            self.controls.commands.remove(0);
        }
        if action == "close" {
            self.close_controls();
            return;
        }
        if action == "pause" {
            self.controls.paused = !self.controls.paused;
            return;
        }
        if action == "run" || action == "run-alternate" {
            let scenario = if action == "run-alternate" {
                match self.shelter.as_ref().map(|l| l.trial) {
                    Some(ShelterTrial::Rain) => "shelter-rain",
                    Some(ShelterTrial::Screen) => "shelter-screen-open-water",
                    Some(ShelterTrial::Firebreak) => "shelter-firebreak-dry",
                    None => return,
                }
            } else {
                value
            };
            if let (Some(ticks), Some(trial)) = (
                shelter_scenario_ticks(scenario),
                ShelterTrial::from_scenario(scenario),
            ) {
                self.install_shelter(trial);
                self.controls.runner = Some(Comparison {
                    scenario: scenario.into(),
                    turn: 0,
                    ticks,
                });
                self.controls.paused = false;
                self.controls.progress = 0.0;
                self.controls.outcome = "running";
                self.controls.result.clear();
            }
            return;
        }
        self.controls.runner = None;
        self.controls.paused = false;
        self.controls.outcome = "idle";
        self.controls.progress = 0.0;
        self.controls.result.clear();
        match action {
            "trial" => match value {
                "rain" => self.pending.trial = Some(ShelterTrial::Rain),
                "screen" => self.pending.trial = Some(ShelterTrial::Screen),
                "firebreak" => self.pending.trial = Some(ShelterTrial::Firebreak),
                "orchard" => self.pending.orchard = true,
                _ => {}
            },
            "reset" => self.pending.reset = true,
            "rain" => self.pending.rain = true,
            "ignite" => self.pending.ignite = true,
            "spray" => self.pending.douse = true,
            "panel" | "pickup" => self.pending.pickup = true,
            "chop" => self.pending.chop = true,
            _ => {}
        }
    }

    pub(super) fn queue_comparison_actions(&mut self) {
        if let Some(run) = &self.controls.runner {
            let [rain, ignite, panel, spray] = shelter_script_actions(&run.scenario, run.turn);
            self.pending.rain |= rain;
            self.pending.ignite |= ignite;
            self.pending.pickup |= panel;
            self.pending.douse |= spray;
        }
    }

    pub(super) fn observe_comparison(&mut self) {
        let Some(run) = self.controls.runner.as_mut() else {
            return;
        };
        run.turn += 1;
        self.controls.progress = run.turn as f32 / run.ticks as f32;
        if run.turn < run.ticks {
            return;
        }
        let run = self.controls.runner.take().unwrap();
        self.controls.paused = true;
        println!("pocket-openworld: checking {}", run.scenario);
        match self.verify_shelter_scenario(&run.scenario) {
            Ok(()) => {
                self.controls.outcome = "passed";
                println!(
                    "pocket-openworld: {} PASS; paused for inspection",
                    run.scenario
                );
                self.controls.result = match run.scenario.as_str() {
                    "shelter-rain" => "PASS: sheltered wood burns; exposed wood stays wet.",
                    "shelter-rain-moved" => "PASS: moving the roof lets rain extinguish the fire.",
                    "shelter-screen" => "PASS: screen protects wet wood; exposed wood ignites.",
                    "shelter-screen-open-water" => {
                        "PASS: moving the screen lets both logs dry, then receive water."
                    }
                    "shelter-firebreak" => "PASS: wet strip stops fire; dry row burns through.",
                    "shelter-firebreak-dry" => "PASS: without water, both rows burn through.",
                    _ => "PASS: comparison completed.",
                }
                .into();
            }
            Err(error) => {
                self.controls.outcome = "failed";
                self.controls.result = "FAIL: comparison did not meet its conditions.".into();
                eprintln!("{error}");
            }
        }
    }

    pub(super) fn controls_cursor_mode(&self) -> CursorMode {
        if self.controls.open {
            CursorMode::Visible
        } else {
            CursorMode::Captured
        }
    }
}

pub fn control_scenario_ticks(name: &str) -> Option<u64> {
    Some(match name {
        "controls-open" => 4,
        "controls-rain" => 906,
        "controls-screen" => 1206,
        "controls-screen-water" => 2496,
        "controls-firebreak" | "controls-firebreak-dry" => 1806,
        "controls-drag" => 8,
        "controls-return" => 12,
        "controls-stop" => 12,
        _ => return None,
    })
}

impl WorldGame {
    /// Coordinate-driven input: semantic names locate the native core's painted
    /// bounds, then ordinary cursor/button edges do the activation.
    pub fn click_control(&self, input: &mut Input, name: &str) -> Result<()> {
        let (x, y, w, h) = self
            .control_bounds(name)
            .with_context(|| format!("control {name} is not painted"))?;
        ensure!(w > 0.0 && h > 0.0, "empty control: {name}");
        input.inject_cursor(x + w * 0.5, y + h * 0.5);
        input.inject_mouse_button(winit::event::MouseButton::Left, true);
        input.inject_mouse_button(winit::event::MouseButton::Left, false);
        Ok(())
    }

    pub fn apply_control_script(&self, input: &mut Input, name: &str, turn: u64) -> Result<()> {
        if name == "controls-stop" {
            input.inject_key(KeyCode::KeyW, true);
            input.inject_key(KeyCode::Tab, turn == 8);
            return Ok(());
        }
        input.inject_key(KeyCode::Tab, turn == 0);
        let trial = match name {
            "controls-rain" => "trial-rain",
            "controls-screen" | "controls-screen-water" => "trial-screen",
            _ => "trial-firebreak",
        };
        if name == "controls-open" {
            return Ok(());
        }
        if name == "controls-drag" {
            if turn == 2 {
                let (x, y, w, h) = self
                    .control_bounds("window-caption")
                    .context("caption missing")?;
                input.inject_cursor(x + w * 0.5, y + h * 0.5);
                input.inject_mouse_button(winit::event::MouseButton::Left, true);
            } else if turn == 3 {
                let cursor = input.cursor().unwrap();
                input.inject_cursor(cursor.x + 130.0, cursor.y + 12.0);
            } else if turn == 4 {
                input.inject_mouse_button(winit::event::MouseButton::Left, false);
            } else if turn == 5 {
                self.click_control(input, "trial-firebreak")?;
            }
            return Ok(());
        }
        if name == "controls-return" {
            input.inject_key(KeyCode::KeyW, turn == 0 || turn >= 8);
            if turn < 6 {
                input.inject_mouse_delta(35.0, 10.0);
            }
            if turn == 6 {
                self.click_control(input, "close")?;
            }
            return Ok(());
        }
        if turn == 2 {
            self.click_control(input, trial)?;
        }
        if turn == 4 {
            self.click_control(
                input,
                if name == "controls-firebreak-dry" || name == "controls-screen-water" {
                    "run-alternate"
                } else {
                    "run-primary"
                },
            )?;
        }
        Ok(())
    }

    pub fn verify_control_scenario(&self, name: &str) -> Result<()> {
        let c = self.control_receipt();
        ensure!(c.error.is_none(), "PocketJS UI error: {:?}", c.error);
        if name == "controls-return" {
            ensure!(
                !c.visible
                    && c.cursor_mode == "captured"
                    && c.commands.iter().any(|s| s == "close"),
                "closing did not return control"
            );
        } else {
            ensure!(
                c.visible && c.cursor_mode == "pointer",
                "overlay did not own the pointer"
            );
            if name == "controls-drag" {
                ensure!(
                    c.commands.iter().any(|s| s == "trial:firebreak"),
                    "clicking after drag failed"
                );
            } else if !matches!(name, "controls-open" | "controls-stop") {
                ensure!(
                    c.outcome == "passed" && c.paused && !c.running && c.progress == 1.0,
                    "comparison did not pass and pause: {c:?}"
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::event::MouseButton;

    fn step(game: &mut WorldGame, input: &mut Input) {
        game.frame(1.0 / 60.0, input);
        game.tick(1.0 / 60.0, input);
        input.end_frame();
    }

    #[test]
    fn opening_controls_stops_motion_without_pushing_touching_bodies() {
        for (collider, mass) in [
            (Collider::Sphere { radius: 0.3 }, 1.0),
            (
                Collider::CapsuleY {
                    radius: 0.3,
                    half_height: 0.2,
                },
                4.0,
            ),
        ] {
            let mut game = WorldGame::new(7);
            game.world.config_mut().gravity = Vec3::ZERO;
            let mut input = Input::default();
            input.inject_key(KeyCode::KeyD, true);
            step(&mut game, &mut input);
            let player = game.world.entity(game.ids.player).unwrap();
            let direction = player.body.unwrap().linear_velocity.normalize();
            assert!(player.body.unwrap().linear_velocity.length() > 3.5);
            let stopped = player.transform.position;
            let phase = game.walk_phase;
            let mut touching =
                EntityBundle::new(Transform::from_translation(stopped + direction * 0.55));
            touching.collider = Some(collider);
            touching.body = Some(Body::dynamic(mass));
            let touching = game.world.spawn(touching);
            input.inject_key(KeyCode::Tab, true);
            step(&mut game, &mut input);
            input.inject_key(KeyCode::Tab, false);
            for _ in 0..3 {
                assert_eq!(game.player_position(), stopped);
                assert_eq!(
                    game.world
                        .entity(game.ids.player)
                        .unwrap()
                        .body
                        .unwrap()
                        .linear_velocity,
                    Vec3::ZERO
                );
                assert!(
                    game.world
                        .entity(touching)
                        .unwrap()
                        .body
                        .unwrap()
                        .linear_velocity
                        .length()
                        < 1e-5
                );
                assert_eq!(game.walk_phase, phase);
                step(&mut game, &mut input);
            }
            // Closing consumes the current frame; held movement resumes next frame.
            input.inject_key(KeyCode::Tab, true);
            step(&mut game, &mut input);
            assert_eq!(game.player_position(), stopped);
            input.inject_key(KeyCode::Tab, false);
            step(&mut game, &mut input);
            assert_ne!(game.player_position(), stopped);
        }
    }

    #[test]
    fn pocketjs_clicks_run_real_comparisons_and_pause_for_inspection() {
        for name in [
            "controls-rain",
            "controls-screen",
            "controls-screen-water",
            "controls-firebreak",
            "controls-firebreak-dry",
        ] {
            let mut game = WorldGame::new(7);
            let mut input = Input::default();
            for turn in 0..control_scenario_ticks(name).unwrap() {
                game.apply_control_script(&mut input, name, turn).unwrap();
                step(&mut game, &mut input);
            }
            game.verify_control_scenario(name).unwrap();
            let frozen = game.world.state_hash();
            step(&mut game, &mut input);
            assert_eq!(frozen, game.world.state_hash());
            game.click_control(&mut input, "reset").unwrap();
            step(&mut game, &mut input);
            assert!(!game.controls.paused);
            assert_eq!(game.control_receipt().outcome, "idle");
            assert!(!game.control_receipt().running);
        }
    }

    #[test]
    fn manual_buttons_use_the_same_water_and_heat_rules_and_close_resumes() {
        let mut game = WorldGame::new_shelter(7);
        let mut input = Input::default();
        game.open_controls().unwrap();
        game.click_control(&mut input, "rain").unwrap();
        step(&mut game, &mut input);
        for _ in 0..420 {
            step(&mut game, &mut input);
        }
        game.click_control(&mut input, "ignite").unwrap();
        step(&mut game, &mut input);
        for _ in 0..58 {
            step(&mut game, &mut input);
        }
        game.verify_shelter_scenario("shelter-rain").unwrap();
        game.click_control(&mut input, "panel").unwrap();
        step(&mut game, &mut input);
        for _ in 0..419 {
            step(&mut game, &mut input);
        }
        game.verify_shelter_scenario("shelter-rain-moved").unwrap();
        game.click_control(&mut input, "trial-screen").unwrap();
        step(&mut game, &mut input);
        step(&mut game, &mut input);
        game.click_control(&mut input, "ignite").unwrap();
        step(&mut game, &mut input);
        for _ in 0..1199 {
            step(&mut game, &mut input);
        }
        game.verify_shelter_scenario("shelter-screen").unwrap();
        game.click_control(&mut input, "spray").unwrap();
        step(&mut game, &mut input);
        for _ in 0..89 {
            step(&mut game, &mut input);
        }
        game.verify_shelter_scenario("shelter-screen-water")
            .unwrap();
        game.click_control(&mut input, "pause").unwrap();
        step(&mut game, &mut input);
        let tick = game.world.tick();
        step(&mut game, &mut input);
        assert_eq!(tick, game.world.tick());
        game.click_control(&mut input, "close").unwrap();
        step(&mut game, &mut input);
        assert!(!game.controls.paused);
        assert!(!game.controls.open);
        assert!(game.world.tick() > tick);
        game.open_controls().unwrap();
        game.click_control(&mut input, "trial-firebreak").unwrap();
        step(&mut game, &mut input);
        step(&mut game, &mut input);
        let count = game.controls.commands.len();
        game.click_control(&mut input, "panel").unwrap();
        step(&mut game, &mut input);
        assert_eq!(
            count,
            game.controls.commands.len(),
            "disabled controls must not activate"
        );
    }

    #[test]
    fn pointer_focus_drag_resize_and_return_do_not_leak_into_gameplay() {
        for scale in [1.0, 2.0] {
            let mut game = WorldGame::new(7);
            game.resized(
                ((960.0 * scale) as u32, (600.0 * scale) as u32),
                scale as f64,
            );
            let original = game.player_position();
            let original_yaw = game.orbit_yaw;
            let mut input = Input::default();
            input.inject_key(KeyCode::KeyW, true);
            input.inject_mouse_delta(90.0, 30.0);
            input.inject_key(KeyCode::Tab, true);
            step(&mut game, &mut input);
            input.inject_key(KeyCode::Tab, false);
            for _ in 0..5 {
                input.inject_mouse_delta(90.0, 30.0);
                step(&mut game, &mut input);
            }
            assert_eq!(original, game.player_position());
            assert_eq!(original_yaw, game.orbit_yaw);
            let before = game.control_bounds("window-caption").unwrap();
            for delta in [80.0, -30.0] {
                let (x, y, w, h) = game.control_bounds("window-caption").unwrap();
                input.inject_cursor(x + w * 0.5, y + h * 0.5);
                input.inject_mouse_button(MouseButton::Left, true);
                step(&mut game, &mut input);
                input.inject_cursor(x + w * 0.5 + delta * scale, y + h * 0.5);
                step(&mut game, &mut input);
                input.inject_mouse_button(MouseButton::Left, false);
                step(&mut game, &mut input);
            }
            let after = game.control_bounds("window-caption").unwrap();
            assert!((after.0 - before.0 - 50.0 * scale).abs() < 2.1);
            game.click_control(&mut input, "trial-rain").unwrap();
            step(&mut game, &mut input);
            step(&mut game, &mut input);
            let (x, y, w, h) = game.control_bounds("ignite").unwrap();
            input.inject_cursor(x + w * 0.5, y + h * 0.5);
            input.inject_mouse_button(MouseButton::Left, true);
            step(&mut game, &mut input);
            input.clear();
            step(&mut game, &mut input);
            input.inject_mouse_button(MouseButton::Left, false);
            step(&mut game, &mut input);
            assert!(
                !game
                    .control_receipt()
                    .commands
                    .iter()
                    .any(|s| s == "ignite")
            );
            game.resized(
                ((1100.0 * scale) as u32, (700.0 * scale) as u32),
                scale as f64,
            );
            step(&mut game, &mut input);
            game.click_control(&mut input, "rain").unwrap();
            step(&mut game, &mut input);
            assert!(game.shelter_receipt().unwrap().rain);
            game.click_control(&mut input, "close").unwrap();
            step(&mut game, &mut input);
            assert_eq!(game.cursor_mode(), CursorMode::Captured);
            let pos = game.player_position();
            input.inject_key(KeyCode::KeyW, true);
            step(&mut game, &mut input);
            assert_ne!(pos, game.player_position());
            input.inject_key(KeyCode::Tab, true);
            step(&mut game, &mut input);
            assert_eq!(game.cursor_mode(), CursorMode::Visible);
        }
    }
}
