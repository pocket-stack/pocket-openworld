//! HUD content for the same PocketJS text/layout renderer as the controls.
use super::*;
use pocket3d_world::IgnitionStatus;
use serde_json::{Value, json};

impl WorldGame {
    pub(super) fn hud_model(&self) -> Value {
        if let Some(receipt) = self.shelter_receipt() {
            let firebreak = receipt.trial == ShelterTrial::Firebreak;
            let pairs = if firebreak {
                [(2, 4), (7, 9)]
            } else {
                [(0, 0), (1, 1)]
            };
            let samples: Vec<_> = pairs.into_iter().enumerate().map(|(side, (start, end))| {
                let sample = &receipt.samples[start];
                let status = match sample.ignition_status {
                    IgnitionStatus::Burning => "Burning",
                    IgnitionStatus::TooWet => "Too wet to ignite",
                    IgnitionStatus::TooCold => "Needs more heat",
                    IgnitionStatus::Exhausted => "Fuel exhausted",
                    IgnitionStatus::Ready => "Ready to ignite",
                    IgnitionStatus::Inert => "Inert",
                };
                let history = match (firebreak, receipt.samples[end].ever_ignited) {
                    (true, true) => "Fire reached the far end",
                    (true, false) => "Far end unburned",
                    (false, true) => "Has ignited",
                    (false, false) => "Has not ignited",
                };
                json!({"label":if firebreak { if side == 0 { "Left / control" } else { "Right / sprinkler" } }
                    else if side == 0 { "Left log" } else { "Right log" },
                    "wet":sample.moisture * 100.0,"temperature":sample.temperature_c,
                    "status":status,"history":history})
            }).collect();
            let panel = match receipt.trial {
                ShelterTrial::Rain => {
                    if receipt.screen_open {
                        "Roof on right"
                    } else {
                        "Roof on left"
                    }
                }
                ShelterTrial::Screen => {
                    if receipt.screen_open {
                        "Screen aside"
                    } else {
                        "Screen in path"
                    }
                }
                ShelterTrial::Firebreak => "Compare again without water in the control window",
            };
            let (w, h) = self.controls.size;
            let (width, height) = (
                w as f32 / self.controls.scale,
                h as f32 / self.controls.scale,
            );
            let labels: Vec<_> = [(-2.0, "Left"), (2.0, "Right")].into_iter().filter_map(|(x, label)| {
                let ndc = self.camera.view_proj(width / height).project_point3(Vec3::new(x, 0.2, if firebreak { 2.2 } else { -1.3 }));
                (ndc.is_finite() && ndc.z > 0.0 && ndc.z < 1.0 && ndc.x.abs() < 1.0 && ndc.y.abs() < 1.0)
                    .then(|| json!({"label":label,"x":(ndc.x+1.0)*width*0.5,"y":(1.0-ndc.y)*height*0.5}))
            }).collect();
            json!({"kind":"lab","title":receipt.trial.title(),"instruction":receipt.trial.instruction(),
                "samples":samples,"labels":labels,
                "footer":format!("Time {:.0}s   Rain {}   {}",receipt.elapsed_seconds,if receipt.rain { "on" } else { "off" },panel)})
        } else {
            let water = water_hud_state(self.water_burst_turns);
            let target = self.last_target.or_else(|| self.nearest_reactive(self.player_position(), 6.0, false))
                .and_then(|id| {
                    let entity = self.world.entity(id)?;
                    let state = entity.reactive_state?;
                    Some(json!({"name":format!("{} #{}",entity.name.as_deref().unwrap_or("Material"),id.0),
                        "temperature":state.temperature_c,"wet":state.moisture*100.0,
                        "fuel":state.fuel*100.0,"burning":state.burning,
                        "cook":state.cook_progress*100.0,"char":state.char_progress*100.0}))
                });
            json!({"kind":"orchard","water":{"spraying":water.spraying,"progress":water.progress},
                "target":target,"message":if self.message_turns > 0 { &self.message } else { "" }})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocket3d::app::CursorMode;
    use winit::event::MouseButton;

    #[test]
    fn passive_hud_refreshes_without_taking_input_and_returns_after_controls() {
        for scale in [1.0, 2.0] {
            let mut game = WorldGame::new(7);
            game.resized(
                ((960.0 * scale) as u32, (600.0 * scale) as u32),
                scale as f64,
            );
            game.refresh_presentation_ui();
            assert!(game.control_bounds("hud-orchard").is_some());
            assert!(game.control_bounds("window").is_none());
            assert_eq!(game.cursor_mode(), CursorMode::Captured);

            // A click over a painted HUD panel still belongs to the game.
            let mut input = Input::default();
            input.inject_cursor(40.0 * scale, 40.0 * scale);
            input.inject_mouse_button(MouseButton::Left, true);
            input.inject_key(KeyCode::KeyW, true);
            let before = game.player_position();
            game.frame(1.0 / 60.0, &input);
            game.tick(1.0 / 60.0, &input);
            game.refresh_presentation_ui();
            assert_ne!(game.player_position(), before);
            assert!(game.control_receipt().commands.is_empty());

            game.open_controls().unwrap();
            assert!(game.control_bounds("game-hud").is_none());
            assert!(game.control_bounds("window").is_some());
            let mut close = Input::default();
            close.inject_key(KeyCode::Tab, true);
            game.frame(1.0 / 60.0, &close);
            game.say("Water burst reached several material volumes; the notification remains readable when the viewport changes.", 90);
            game.refresh_presentation_ui();
            assert!(game.control_bounds("hud-message").is_some());
            assert!(game.control_bounds("window").is_none());

            game.install_shelter(ShelterTrial::Rain);
            game.resized(
                ((800.0 * scale) as u32, (600.0 * scale) as u32),
                scale as f64,
            );
            game.refresh_presentation_ui();
            assert!(game.control_bounds("hud-orchard").is_none());
            assert!(game.control_bounds("hud-lab").is_some());
            assert!(game.control_bounds("hud-samples").is_some());
            assert_eq!(game.cursor_mode(), CursorMode::Captured);
            assert!(game.control_receipt().error.is_none());
        }
    }
}
