//! Application recipes and controls for three small, repeatable chemistry trials.
//! Every temperature, water dose and ignition result is resolved by pocket3d-world.

use super::*;
use pocket3d_world::{
    IgnitionStatus, Rainfall, TransportChannel, TransportReport, TransportSurface, WaterEmission,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ShelterTrial {
    Rain,
    Screen,
    Firebreak,
}

impl ShelterTrial {
    pub fn from_scenario(name: &str) -> Option<Self> {
        if name.starts_with("shelter-rain") {
            Some(Self::Rain)
        } else if name.starts_with("shelter-screen") || name.starts_with("shelter-dry") {
            Some(Self::Screen)
        } else if name.starts_with("shelter-firebreak") {
            Some(Self::Firebreak)
        } else {
            None
        }
    }
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Rain => "01  RAIN & SHELTER",
            Self::Screen => "02  HEAT & WATER SCREEN",
            Self::Firebreak => "03  WET FIREBREAK",
        }
    }
    pub(super) fn instruction(self) -> &'static str {
        match self {
            Self::Rain => "Rain wets exposed wood; a moving roof changes which log stays dry.",
            Self::Screen => {
                "The same screen intercepts heat and water. Move it to change exposure."
            }
            Self::Firebreak => "Wet the middle strip to stop fire. Compare with the dry row.",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ShelterSampleReceipt {
    pub id: u64,
    pub label: String,
    pub temperature_c: f32,
    pub moisture: f32,
    pub burning: bool,
    pub ever_ignited: bool,
    pub ignition_status: IgnitionStatus,
    pub water_received: f32,
    pub heat_received: f32,
    pub heat_intercepted: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShelterReceipt {
    pub trial: ShelterTrial,
    pub rain: bool,
    pub screen_open: bool,
    pub elapsed_seconds: f32,
    pub rainfall: Option<Rainfall>,
    pub panel: Option<ShelterPanelReceipt>,
    pub samples: Vec<ShelterSampleReceipt>,
    pub transport: TransportReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShelterPanelReceipt {
    pub id: u64,
    pub transform: Transform,
    pub surface: TransportSurface,
}

pub(super) struct ShelterLab {
    pub(super) trial: ShelterTrial,
    samples: Vec<EntityId>,
    heaters: Vec<EntityId>,
    panel: Option<EntityId>,
    panel_closed: Vec3,
    panel_open: Vec3,
    screen_open: bool,
    overview: bool,
    rain: bool,
    spray_turns: u16,
    spray_paths: Vec<WaterEmission>,
    transport: TransportReport,
}

fn ground(x: f32, z: f32) -> f32 {
    OrchardEnvironment::height_at(Vec2::new(x, z))
}

fn log_bundle(x: f32, z: f32, name: &str, moisture: f32) -> EntityBundle {
    let mut bundle = EntityBundle::new(Transform {
        position: Vec3::new(x, ground(x, z) + 0.48, z),
        rotation: Quat::from_rotation_z(FRAC_PI_2),
        scale: Vec3::ONE,
    })
    .named(name)
    .tagged("wood");
    bundle.collider = Some(Collider::CapsuleY {
        radius: 0.26,
        half_height: 0.55,
    });
    bundle.reactive_material = Some(wood_material());
    bundle.reactive_state = Some(ReactiveState::new(24.0, moisture, 3.0));
    bundle
}

impl WorldGame {
    pub fn verify_shelter_scenario(&self, scenario: &str) -> Result<()> {
        let receipt = self
            .shelter_receipt()
            .context("shelter scenario has no lab state")?;
        let ledger = &receipt.transport;
        let tolerance = 0.001 + ledger.water_emitted * 0.00002;
        ensure!(
            (ledger.water_emitted
                - ledger.water_retained
                - ledger.water_runoff
                - ledger.water_escaped)
                .abs()
                <= tolerance,
            "transport water budget failed: {ledger:?}"
        );
        ensure!(
            receipt
                .samples
                .iter()
                .all(|s| s.temperature_c.is_finite() && (0.0..=1.0).contains(&s.moisture)),
            "non-finite temperature or invalid moisture"
        );
        let a = &receipt.samples[0];
        let b = &receipt.samples[1];
        let passed = match scenario {
            "shelter-rain" => a.burning && a.moisture < 0.2 && !b.ever_ignited && b.moisture > 0.8,
            "shelter-rain-moved" => {
                a.ever_ignited
                    && !a.burning
                    && a.moisture > 0.8
                    && !b.ever_ignited
                    && b.moisture > 0.7
                    && receipt.screen_open
            }
            "shelter-screen" => {
                !a.ever_ignited
                    && a.moisture > self.world.config().ignition_max_moisture
                    && a.heat_intercepted > 100.0
                    && b.burning
                    && b.moisture < 0.15
            }
            "shelter-screen-water" => {
                !a.ever_ignited
                    && a.moisture > self.world.config().ignition_max_moisture
                    && a.water_received < 1e-5
                    && b.ever_ignited
                    && !b.burning
                    && b.moisture > 0.85
                    && b.water_received > 1.0
            }
            "shelter-screen-open-water" => {
                receipt.screen_open
                    && receipt.samples.iter().all(|s| {
                        s.ever_ignited && !s.burning && s.moisture > 0.85 && s.water_received > 1.0
                    })
            }
            "shelter-dry" => {
                receipt.screen_open
                    && ledger.water_evaporated > 1.2
                    && receipt
                        .samples
                        .iter()
                        .all(|s| s.burning && s.moisture < 0.15)
            }
            "shelter-firebreak" => {
                receipt.samples[..5].iter().all(|s| s.ever_ignited)
                    && receipt.samples[5].ever_ignited
                    && receipt.samples[6].ever_ignited
                    && receipt.samples[7].moisture > 0.55
                    && receipt.samples[7..].iter().all(|s| !s.ever_ignited)
            }
            "shelter-firebreak-dry" => receipt.samples.iter().all(|s| s.ever_ignited),
            _ => false,
        };
        ensure!(passed, "{scenario} acceptance failed: {receipt:#?}");
        Ok(())
    }

    pub fn new_shelter(seed: u64) -> Self {
        let mut game = Self::new(seed);
        game.install_shelter(ShelterTrial::Rain);
        game
    }

    pub(super) fn install_shelter(&mut self, trial: ShelterTrial) {
        let player = self
            .world
            .entity(self.ids.player)
            .cloned()
            .expect("player recipe");
        self.world = World::with_seed(self.seed);
        let mut player_bundle = EntityBundle::new(player.transform)
            .named("explorer")
            .tagged("player");
        player_bundle.body = player.body;
        player_bundle.collider = player.collider;
        player_bundle.transform.position =
            player_capsule_center(Vec2::new(0.0, 5.5), ground(0.0, 5.5));
        self.ids.player = self.world.spawn(player_bundle);
        self.ids.trees.clear();
        self.ids.grass.clear();
        self.ids.camp_logs.clear();
        self.ids.fire = EntityId(0);
        self.ids.damp_log = EntityId(0);
        self.decorations.clear();
        self.receipts = RuntimeReceipts::default();
        self.pending = PendingActions::default();
        self.held_apple = None;
        self.axe_swing_turns = 0;
        self.ember_cast_turns = 0;
        self.water_burst_turns = 0;
        self.water_pose_turns = 0;
        self.walk_phase = 0.0;
        self.orbit_yaw = 0.0;
        self.environment = OrchardEnvironment::default();
        self.world.config_mut().surface_evaporation_rate = 1.0;
        if trial == ShelterTrial::Screen {
            self.world.config_mut().ambient_exchange = 1.2;
        }
        let mut lab = ShelterLab {
            trial,
            samples: Vec::new(),
            heaters: Vec::new(),
            panel: None,
            panel_closed: Vec3::ZERO,
            panel_open: Vec3::ZERO,
            screen_open: false,
            overview: true,
            rain: false,
            spray_turns: 0,
            spray_paths: Vec::new(),
            transport: TransportReport::default(),
        };
        match trial {
            ShelterTrial::Rain | ShelterTrial::Screen => {
                for (x, label) in [(-2.0, "LEFT LOG"), (2.0, "RIGHT LOG")] {
                    lab.samples.push(self.world.spawn(log_bundle(
                        x,
                        0.0,
                        label,
                        if trial == ShelterTrial::Screen {
                            0.82
                        } else {
                            0.06
                        },
                    )));
                    if trial == ShelterTrial::Screen {
                        let mut heater = EntityBundle::new(Transform::from_translation(Vec3::new(
                            x,
                            ground(x, 2.3) + 0.36,
                            2.3,
                        )))
                        .named("charcoal brazier")
                        .tagged("fire");
                        heater.collider = Some(Collider::Sphere { radius: 0.3 });
                        heater.reactive_material = Some(ReactiveMaterial {
                            heat_capacity: 10.0,
                            burn_rate: 0.2,
                            heat_output: 18_000.0,
                            ..flame_material()
                        });
                        heater.reactive_state = Some(ReactiveState::new(24.0, 0.0, 18.0));
                        lab.heaters.push(self.world.spawn(heater));
                    }
                }
                let is_roof = trial == ShelterTrial::Rain;
                lab.panel_closed = if is_roof {
                    Vec3::new(-2.0, 2.5, 0.0)
                } else {
                    Vec3::new(-2.0, 1.2, 1.1)
                };
                lab.panel_open = if is_roof {
                    lab.panel_closed + Vec3::X * 4.0
                } else {
                    lab.panel_closed - Vec3::X * 3.2
                };
                let mut panel = EntityBundle::new(Transform::from_translation(lab.panel_closed))
                    .named(if is_roof {
                        "sliding roof"
                    } else {
                        "sliding screen"
                    });
                panel.transport_surface = Some(TransportSurface {
                    half_extents: if is_roof {
                        Vec3::new(1.25, 0.08, 1.05)
                    } else {
                        Vec3::new(1.15, 1.1, 0.08)
                    },
                    water_transmission: 0.0,
                    heat_transmission: 0.0,
                });
                lab.panel = Some(self.world.spawn(panel));
            }
            ShelterTrial::Firebreak => {
                self.world.config_mut().reaction_radius = 1.2;
                self.world.config_mut().ambient_exchange = 0.12;
                for (row, x) in [(0, -2.0), (1, 2.0)] {
                    for index in 0..5 {
                        let z = 1.4 - index as f32 * 1.3;
                        let mut patch = EntityBundle::new(Transform::from_translation(Vec3::new(
                            x,
                            ground(x, z) + 0.20,
                            z,
                        )))
                        .named(format!(
                            "{} {}",
                            if row == 0 { "DRY" } else { "WATERED" },
                            index + 1
                        ))
                        .tagged("grass");
                        patch.collider = Some(Collider::Sphere { radius: 0.55 });
                        patch.reactive_material = Some(ReactiveMaterial {
                            heat_capacity: 0.7,
                            ..grass_material()
                        });
                        patch.reactive_state = Some(ReactiveState::new(24.0, 0.02, 0.55));
                        lab.samples.push(self.world.spawn(patch));
                    }
                }
            }
        }
        self.message = trial.instruction().into();
        self.message_turns = 0;
        self.shelter = Some(lab);
    }

    pub fn shelter_receipt(&self) -> Option<ShelterReceipt> {
        let lab = self.shelter.as_ref()?;
        let samples = lab
            .samples
            .iter()
            .filter_map(|&id| {
                let entity = self.world.entity(id)?;
                let state = entity.reactive_state?;
                Some(ShelterSampleReceipt {
                    id: id.0,
                    label: entity.name.clone().unwrap_or_default(),
                    temperature_c: state.temperature_c,
                    moisture: state.moisture,
                    burning: state.burning,
                    ever_ignited: self.receipts.ignited.contains(&id),
                    ignition_status: self.world.ignition_status(id),
                    water_received: lab
                        .transport
                        .water
                        .iter()
                        .filter(|t| t.target == id)
                        .map(|t| t.delivered)
                        .sum(),
                    heat_received: lab
                        .transport
                        .heat
                        .iter()
                        .filter(|t| t.target == id)
                        .map(|t| t.received)
                        .sum(),
                    heat_intercepted: lab
                        .transport
                        .heat
                        .iter()
                        .filter(|t| t.target == id)
                        .map(|t| t.intercepted)
                        .sum(),
                })
            })
            .collect();
        Some(ShelterReceipt {
            trial: lab.trial,
            rain: lab.rain,
            screen_open: lab.screen_open,
            elapsed_seconds: self.world.tick() as f32 / 60.0,
            rainfall: self.environment.rain,
            panel: lab.panel.and_then(|id| {
                let entity = self.world.entity(id)?;
                Some(ShelterPanelReceipt {
                    id: id.0,
                    transform: entity.transform,
                    surface: entity.transport_surface?,
                })
            }),
            samples,
            transport: lab.transport.clone(),
        })
    }

    pub(super) fn tick_shelter(&mut self, dt: f32, input: &Input) {
        if std::mem::take(&mut self.pending.reset) {
            let trial = self.shelter.as_ref().unwrap().trial;
            self.install_shelter(trial);
        }
        self.move_player(input, dt);
        let mut lab = self.shelter.take().unwrap();
        if std::mem::take(&mut self.pending.rain) {
            lab.rain = !lab.rain;
        }
        if std::mem::take(&mut self.pending.view) {
            lab.overview = !lab.overview;
        }
        if std::mem::take(&mut self.pending.pickup) {
            lab.screen_open = !lab.screen_open;
        }
        if let Some(panel) = lab.panel.and_then(|id| self.world.entity_mut(id)) {
            let destination = if lab.screen_open {
                lab.panel_open
            } else {
                lab.panel_closed
            };
            let delta = destination - panel.transform.position;
            panel.transform.position += delta.normalize_or_zero() * delta.length().min(dt * 4.0);
        }
        self.environment.humidity = if lab.rain { 0.8 } else { 0.025 };
        self.environment.rain = lab.rain.then_some(Rainfall {
            min: Vec3::new(-4.0, -0.5, -2.0),
            max: Vec3::new(4.0, 5.0, 3.0),
            spacing: 0.18,
            rate: 0.45,
            temperature_c: 18.0,
        });
        if std::mem::take(&mut self.pending.ignite) {
            let targets: Vec<_> = match lab.trial {
                ShelterTrial::Rain => lab.samples.clone(),
                ShelterTrial::Screen => lab.heaters.clone(),
                ShelterTrial::Firebreak => vec![lab.samples[0], lab.samples[5]],
            };
            for target in targets {
                self.world.queue_interaction(Interaction::Ignite {
                    target,
                    energy: if lab.trial == ShelterTrial::Screen {
                        5000.0
                    } else {
                        750.0
                    },
                });
            }
        }
        if std::mem::take(&mut self.pending.douse) {
            lab.spray_turns = 120;
        }
        lab.spray_paths.clear();
        if lab.spray_turns > 0 {
            lab.spray_turns -= 1;
            match lab.trial {
                ShelterTrial::Rain | ShelterTrial::Screen => {
                    for x in [-2.0, 2.0] {
                        for offset in [-0.5, -0.25, 0.0, 0.25, 0.5] {
                            let nozzle = Vec3::new(x, 1.6, 3.1);
                            let target = Vec3::new(x + offset, ground(x, 0.0) + 0.48, -0.6);
                            lab.spray_paths.push(WaterEmission {
                                points: vec![nozzle, target],
                                amount: 0.008,
                                temperature_c: 18.0,
                                source: None,
                            });
                        }
                    }
                }
                ShelterTrial::Firebreak => {
                    for x in [1.7, 2.0, 2.3] {
                        for z in [-0.9, -1.2, -1.5] {
                            lab.spray_paths.push(WaterEmission {
                                points: vec![Vec3::new(x, 2.0, z), Vec3::new(x, -0.4, z)],
                                amount: 0.004,
                                temperature_c: 18.0,
                                source: None,
                            });
                        }
                    }
                }
            }
            for path in &lab.spray_paths {
                self.world
                    .queue_interaction(Interaction::Water(path.clone()));
            }
        }
        let report = self.world.step(&self.environment);
        self.observe_report(&report);
        accumulate_transport(&mut lab.transport, &report.transport);
        self.pending.chop = false;
        self.shelter = Some(lab);
    }

    pub(super) fn rebuild_shelter_scene(&mut self, time: f32, size: (u32, u32)) {
        let Some(assets) = self.assets.clone() else {
            return;
        };
        self.scene.models.clear();
        self.scene.sprites.clear();
        self.scene.beams.clear();
        self.scene.time = time;
        self.scene.models.push(art::instance(
            &assets.ground,
            Mat4::IDENTITY,
            [0.82, 0.92, 0.85, 1.0],
        ));
        let lab = self.shelter.as_ref().unwrap();
        let trial = lab.trial;
        let samples = lab.samples.clone();
        for &id in &samples {
            let entity = self.world.entity(id).unwrap();
            let state = entity.reactive_state.unwrap();
            let moisture = state.moisture;
            let charred = state
                .char_progress
                .max(if state.burned_out { 1.0 } else { 0.0 });
            let is_grass = trial == ShelterTrial::Firebreak;
            let base = if is_grass {
                [0.26, 0.57, 0.12, 1.0]
            } else {
                [0.62, 0.35, 0.14, 1.0]
            };
            let tint = mix4(
                mix4(base, [0.12, 0.24, 0.22, 1.0], moisture * 0.85),
                [0.06, 0.035, 0.025, 1.0],
                charred,
            );
            let matrix = if is_grass {
                let mut transform = entity.transform;
                transform.position.y = ground(transform.position.x, transform.position.z);
                transform.scale = Vec3::new(0.72, 0.75 * (1.0 - charred * 0.65), 0.72);
                transform_matrix(transform)
            } else {
                transform_matrix(entity.transform) * Mat4::from_scale(Vec3::new(0.26, 1.62, 0.26))
            };
            self.scene.models.push(art::instance(
                if is_grass {
                    &assets.grass
                } else {
                    &assets.trunk
                },
                matrix,
                tint,
            ));
            // A blue ground sheen accompanies the persistent moisture reading.
            if moisture > 0.1 {
                self.scene.models.push(art::instance(
                    &assets.shadow,
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.85, 1.0, 0.6),
                        Quat::IDENTITY,
                        Vec3::new(
                            entity.transform.position.x,
                            ground(entity.transform.position.x, entity.transform.position.z)
                                + 0.025,
                            entity.transform.position.z,
                        ),
                    ),
                    [0.12, 0.42, 0.51, moisture * 0.7],
                ));
            }
            if state.temperature_c > 70.0 && moisture > 0.08 {
                for index in 0..3 {
                    let phase = (time * 0.65 + index as f32 * 0.33).fract();
                    self.scene.sprites.push(Sprite {
                        pos: entity.transform.position
                            + Vec3::new((index as f32 - 1.0) * 0.23, 0.4 + phase * 0.9, 0.0),
                        size: 0.16 + phase * 0.16,
                        color: [0.77, 0.88, 0.9, (1.0 - phase) * 0.3],
                    });
                }
            }
        }
        for &id in &lab.heaters {
            let position = self.world.entity(id).unwrap().transform.position;
            self.scene.models.push(art::instance(
                &assets.stump,
                Mat4::from_scale_rotation_translation(
                    Vec3::new(0.45, 0.35, 0.45),
                    Quat::IDENTITY,
                    position,
                ),
                [0.24, 0.22, 0.19, 1.0],
            ));
        }
        if let Some(panel) = lab.panel.and_then(|id| self.world.entity(id)) {
            let surface = panel.transport_surface.unwrap();
            let tint = if trial == ShelterTrial::Rain {
                [0.20, 0.40, 0.42, 1.0]
            } else {
                [0.34, 0.43, 0.5, 1.0]
            };
            self.scene.models.push(art::instance(
                &assets.panel,
                transform_matrix(panel.transform) * Mat4::from_scale(surface.half_extents * 2.0),
                tint,
            ));
            if trial == ShelterTrial::Rain {
                for x in [-1.12, 1.12] {
                    for z in [-0.9, 0.9] {
                        let p = panel.transform.position + Vec3::new(x, 0.0, z);
                        let h = p.y - ground(p.x, p.z);
                        self.scene.models.push(art::instance(
                            &assets.panel,
                            Mat4::from_scale_rotation_translation(
                                Vec3::new(0.07, h, 0.07),
                                Quat::IDENTITY,
                                p - Vec3::Y * h * 0.5,
                            ),
                            [0.38, 0.30, 0.20, 1.0],
                        ));
                    }
                }
            }
        }
        // Visible fixtures make the one-key paired spray a world-space action.
        if trial != ShelterTrial::Firebreak {
            for x in [-2.0, 2.0] {
                self.scene.models.push(art::instance(
                    &assets.panel,
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.09, 1.45, 0.09),
                        Quat::IDENTITY,
                        Vec3::new(x, 0.85, 3.1),
                    ),
                    [0.12, 0.42, 0.62, 1.0],
                ));
                self.scene.models.push(art::instance(
                    &assets.panel,
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.18, 0.14, 0.30),
                        Quat::IDENTITY,
                        Vec3::new(x, 1.58, 3.02),
                    ),
                    [0.15, 0.52, 0.73, 1.0],
                ));
            }
        } else {
            // A visible overhead sprinkler supplies the right-hand test strip.
            for (position, scale) in [
                (Vec3::new(3.0, 1.0, -1.2), Vec3::new(0.08, 2.0, 0.08)),
                (Vec3::new(2.5, 2.04, -1.2), Vec3::new(1.08, 0.08, 0.08)),
                (Vec3::new(2.0, 2.04, -1.2), Vec3::new(0.68, 0.08, 0.68)),
            ] {
                self.scene.models.push(art::instance(
                    &assets.panel,
                    Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, position),
                    [0.15, 0.52, 0.73, 1.0],
                ));
            }
        }
        for path in &lab.spray_paths {
            let mut remaining = self.world.water_path_distance(path);
            for pair in path.points.windows(2) {
                let length = pair[0].distance(pair[1]);
                let endpoint = pair[0].lerp(pair[1], (remaining / length).clamp(0.0, 1.0));
                water_segment(
                    &mut self.scene,
                    &assets.branch,
                    pair[0],
                    endpoint,
                    0.025,
                    [0.15, 0.66, 0.90, 0.9],
                );
                remaining -= length;
                if remaining <= 0.0 {
                    break;
                }
            }
        }
        if lab.rain {
            for index in 0..110 {
                let x = hash_signed(91, index) * 4.0;
                let z = hash01(92, index) * 5.0 - 2.0;
                let top = Vec3::new(x, 5.0, z);
                let bottom = Vec3::new(x, ground(x, z), z);
                let exposure = self
                    .world
                    .exposure(top, bottom, TransportChannel::Water, &[]);
                let end = exposure
                    .hits
                    .first()
                    .map_or(bottom, |hit| top.lerp(bottom, hit.fraction));
                let fraction = (time * 1.9 + hash01(93, index)).fract();
                let point = top.lerp(end, fraction);
                water_segment(
                    &mut self.scene,
                    &assets.branch,
                    point,
                    point.lerp(end, 0.12),
                    0.012,
                    [0.18, 0.48, 0.73, 0.82],
                );
            }
        }
        let overview = lab.overview;
        self.render_player(&assets, time);
        self.render_combustion(&assets, time);
        if overview {
            self.camera.pos = Vec3::new(7.0, 7.0, 11.5);
            self.camera.look_at(Vec3::new(
                0.0,
                0.65,
                if trial == ShelterTrial::Firebreak {
                    -1.0
                } else {
                    0.6
                },
            ));
            // Keep the comparison in the visible area beside the floating
            // controls. The native UI supplies its painted window bounds.
            if self.controls.open
                && let Some((x, _, w, _)) = self.control_bounds("window")
            {
                let viewport = size.0 as f32;
                let left = x + w * 0.5 < viewport * 0.5;
                let covered = if left { x + w } else { viewport - x }.min(viewport * 0.55);
                let target = Vec3::new(
                    0.0,
                    0.65,
                    if trial == ShelterTrial::Firebreak {
                        -1.0
                    } else {
                        0.6
                    },
                );
                let direction = (target - self.camera.pos).normalize();
                let right = direction.cross(Vec3::Y).normalize();
                let half_width = self.camera.pos.distance(target)
                    * (self.camera.fov_y * 0.5).tan()
                    * size.0 as f32
                    / size.1 as f32;
                let offset =
                    right * half_width * covered / viewport * if left { -1.0 } else { 1.0 };
                self.camera.pos += offset;
                self.camera.look_at(target + offset);
            }
        } else {
            self.update_camera();
        }
        self.hud.clear();
    }
}

fn accumulate_transport(total: &mut TransportReport, step: &TransportReport) {
    total.water_emitted += step.water_emitted;
    total.water_retained += step.water_retained;
    total.water_runoff += step.water_runoff;
    total.water_escaped += step.water_escaped;
    total.water_evaporated += step.water_evaporated;
    for transfer in &step.water {
        if let Some(old) = total
            .water
            .iter_mut()
            .find(|old| old.target == transfer.target && old.source == transfer.source)
        {
            old.delivered += transfer.delivered;
            old.retained += transfer.retained;
            old.runoff += transfer.runoff;
        } else {
            total.water.push(transfer.clone());
        }
    }
    for transfer in &step.heat {
        if let Some(old) = total.heat.iter_mut().find(|old| {
            old.source == transfer.source
                && old.target == transfer.target
                && old.blockers == transfer.blockers
        }) {
            old.sent += transfer.sent;
            old.received += transfer.received;
            old.intercepted += transfer.intercepted;
        } else {
            total.heat.push(transfer.clone());
        }
    }
}

pub(super) fn panel_mesh() -> art::Mesh {
    let mut mesh = art::MeshBuilder::new();
    for (normal, u, v) in [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (-Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (-Vec3::Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (-Vec3::Z, Vec3::Y, Vec3::X),
    ] {
        let c = normal * 0.5;
        let u = u * 0.5;
        let v = v * 0.5;
        mesh.quad_flat(
            [c - u - v, c + u - v, c + u + v, c - u + v],
            [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
        );
    }
    mesh.build()
}

fn water_segment(
    scene: &mut Scene,
    asset: &Arc<ModelAsset>,
    a: Vec3,
    b: Vec3,
    radius: f32,
    color: [f32; 4],
) {
    let delta = b - a;
    let length = delta.length();
    if length < 1e-5 {
        return;
    }
    let mut instance = art::instance(
        asset,
        Mat4::from_scale_rotation_translation(
            Vec3::new(radius, length, radius),
            Quat::from_rotation_arc(Vec3::Y, delta / length),
            (a + b) * 0.5,
        ),
        color,
    );
    instance.lit = 0.0;
    scene.models.push(instance);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIOS: &[(&str, u64)] = &[
        ("shelter-rain", 480),
        ("shelter-rain-moved", 900),
        ("shelter-screen", 1200),
        ("shelter-screen-water", 1290),
        ("shelter-dry", 2400),
        ("shelter-screen-open-water", 2490),
        ("shelter-firebreak", 1800),
        ("shelter-firebreak-dry", 1800),
    ];

    fn step(game: &mut WorldGame, input: &mut Input) {
        game.frame(1.0 / 60.0, input);
        game.tick(1.0 / 60.0, input);
        input.end_frame();
    }

    #[test]
    fn manual_controls_produce_the_rain_screen_drying_and_firebreak_comparisons() {
        for &(name, ticks) in SCENARIOS {
            let mut game = WorldGame::new_shelter(7);
            let mut input = Input::default();
            for turn in 0..ticks {
                crate::apply_scenario_script(&mut input, name, turn);
                step(&mut game, &mut input);
            }
            game.verify_shelter_scenario(name).unwrap();
        }
    }

    #[test]
    fn reset_and_trial_switch_remove_previous_water_heat_and_controls() {
        let mut game = WorldGame::new_shelter(19);
        let mut input = Input::default();
        for turn in 0..480 {
            crate::apply_scenario_script(&mut input, "shelter-rain", turn);
            step(&mut game, &mut input);
        }
        input.inject_key(KeyCode::KeyR, true);
        step(&mut game, &mut input);
        let mut fresh = WorldGame::new_shelter(19);
        step(&mut fresh, &mut Input::default());
        assert_eq!(game.world.snapshot(), fresh.world.snapshot());
        assert_eq!(
            game.shelter_receipt().unwrap().transport,
            fresh.shelter_receipt().unwrap().transport
        );
        input.inject_key(KeyCode::KeyR, false);
        input.inject_key(KeyCode::Digit3, true);
        step(&mut game, &mut input);
        let receipt = game.shelter_receipt().unwrap();
        assert_eq!(receipt.trial, ShelterTrial::Firebreak);
        assert!(!receipt.rain && !receipt.screen_open);
        assert_eq!(receipt.samples.len(), 10);
        assert!(receipt.samples.iter().all(|s| !s.ever_ignited));
        input.inject_key(KeyCode::Digit3, false);
        input.inject_key(KeyCode::Digit0, true);
        step(&mut game, &mut input);
        assert!(game.shelter.is_none());
        assert_eq!(game.ids.trees.len(), 3);
        assert!(game.environment.rain.is_none());
    }
}
