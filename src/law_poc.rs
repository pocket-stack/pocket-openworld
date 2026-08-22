use std::collections::{BTreeMap, BTreeSet};

use glam::{Mat4, Quat, Vec3};
use pocket3d_world::{
    Attachment, Body, BodyMode, Collider, EntityBundle, EntityId, FieldBodyProbe, HiddenRelation,
    HiddenState, HiddenValue, Interaction, LawEvent, LawProjection, PhysicalSurface, SpatialField,
    StepReport, Structure, Transform, World, WorldLawRuntime, spherical_barrier_projection,
};
use serde::Serialize;

const IDENTITY_FIELD: &str = "eva.identity-boundary";
const SYNC_RELATION: &str = "eva.synchronized-with";
const EMBODIED_RELATION: &str = "eva.embodied-by";
const PATHS_RELATION: &str = "titan.connected-via-paths";
const FIELD_RADIUS: f32 = 3.4;
const EVA_FIELD_BUDGET: f32 = 920.0;
const EVA_INTERFERENCE_BUDGET: f32 = 790.0;
const TITAN_REGEN_TURNS: u64 = 84;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LawPocMode {
    EvaAtField,
    TitanPaths,
    Crossover,
}

impl LawPocMode {
    pub fn from_scenario(scenario: &str) -> Option<Self> {
        match scenario {
            "eva-at-field" => Some(Self::EvaAtField),
            "titan-paths" => Some(Self::TitanPaths),
            "world-law-crossover" => Some(Self::Crossover),
            _ => None,
        }
    }

    pub fn scenario(self) -> &'static str {
        match self {
            Self::EvaAtField => "eva-at-field",
            Self::TitanPaths => "titan-paths",
            Self::Crossover => "world-law-crossover",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PocModelKind {
    Sphere,
    Cylinder,
    Disc,
    EvaUnit01,
    ColossalTitanBody,
    ColossalTitanRightArm,
}

#[derive(Clone, Copy, Debug)]
pub struct PocModel {
    pub kind: PocModelKind,
    pub transform: Mat4,
    pub tint: [f32; 4],
    pub lit: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PocBeam {
    pub a: Vec3,
    pub b: Vec3,
    pub width: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct PocSprite {
    pub position: Vec3,
    pub size: f32,
    pub color: [f32; 4],
}

#[derive(Default)]
pub struct PocPresentation {
    pub models: Vec<PocModel>,
    pub beams: Vec<PocBeam>,
    pub sprites: Vec<PocSprite>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TimedLawEvent {
    pub tick: u64,
    pub event: LawEvent,
}

#[derive(Clone, Debug, Serialize)]
pub struct EvaLawReceipt {
    pub pilot: u64,
    pub soul: u64,
    pub unit: u64,
    pub synchronization: f32,
    pub field_budget: f32,
    pub interference_budget: f32,
    pub blocked_projectiles: Vec<u64>,
    pub penetrated_projectiles: Vec<u64>,
    pub ordinary_matter_blocked: bool,
    pub local_interference_observed: bool,
    pub interference_enabled_penetration: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TitanLawReceipt {
    pub host: u64,
    pub paths: u64,
    pub trigger_had_injury: bool,
    pub trigger_had_intent: bool,
    pub trigger_had_power: bool,
    pub trigger_had_paths: bool,
    pub avatar_materialized: bool,
    pub severed_limb: Option<u64>,
    pub severed_limb_fell: bool,
    pub regenerated_limb: Option<u64>,
    pub regeneration_progress: f32,
    pub morphology_restored: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossoverLawReceipt {
    pub field_source: u64,
    pub blocked_body: u64,
    pub penetrating_body: u64,
    pub blocked_body_energy: f32,
    pub penetrating_body_energy: f32,
    pub field_budget: f32,
    pub low_energy_body_blocked: bool,
    pub high_energy_body_penetrated: bool,
    pub crossover_specific_rule_count: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct LawPocReceipt {
    pub schema: &'static str,
    pub scenario: &'static str,
    pub physics_tick: u64,
    pub law_tick: u64,
    pub state_hash: String,
    pub hidden_states: Vec<HiddenState>,
    pub relations: Vec<HiddenRelation>,
    pub fields: Vec<SpatialField>,
    pub events: Vec<TimedLawEvent>,
    pub eva: Option<EvaLawReceipt>,
    pub titan: Option<TitanLawReceipt>,
    pub crossover: Option<CrossoverLawReceipt>,
    pub acceptance_passed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TitanPartKind {
    Torso,
    Head,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

#[derive(Clone, Copy, Debug)]
struct TitanPart {
    entity: EntityId,
    kind: TitanPartKind,
    materialization: f32,
    regeneration: bool,
}

#[derive(Default)]
struct EvaState {
    unit: Option<EntityId>,
    soul: Option<EntityId>,
    pilot: Option<EntityId>,
    interferer: Option<EntityId>,
    projectiles: Vec<EntityId>,
    blocked: BTreeSet<EntityId>,
    penetrated: BTreeSet<EntityId>,
    interference_observed: bool,
}

#[derive(Default)]
struct TitanState {
    host: Option<EntityId>,
    paths: Option<EntityId>,
    torso: Option<EntityId>,
    parts: Vec<TitanPart>,
    avatar_started_tick: Option<u64>,
    severed_limb: Option<EntityId>,
    severed_start_y: Option<f32>,
    regenerated_limb: Option<EntityId>,
    regeneration_started_tick: Option<u64>,
    trigger_had_injury: bool,
    trigger_had_intent: bool,
    trigger_had_power: bool,
    trigger_had_paths: bool,
}

#[derive(Default)]
struct CrossoverState {
    source: Option<EntityId>,
    blocked_body: Option<EntityId>,
    penetrating_body: Option<EntityId>,
    blocked: BTreeSet<EntityId>,
    penetrated: BTreeSet<EntityId>,
    initial_energy: BTreeMap<EntityId, f32>,
}

pub struct LawPoc {
    mode: LawPocMode,
    ground_y: f32,
    laws: WorldLawRuntime,
    events: Vec<TimedLawEvent>,
    eva: EvaState,
    titan: TitanState,
    crossover: CrossoverState,
}

impl LawPoc {
    pub fn build_world(
        mode: LawPocMode,
        seed: u64,
        ground_y: f32,
        player_height: f32,
    ) -> (World, EntityId, Self) {
        let mut world = World::with_seed(seed);
        let observer = match mode {
            LawPocMode::EvaAtField => Vec3::new(-1.5, ground_y, 10.5),
            LawPocMode::TitanPaths => Vec3::new(8.5, ground_y, 10.0),
            LawPocMode::Crossover => Vec3::new(-0.5, ground_y, 11.5),
        };
        let mut player = EntityBundle::new(Transform::from_translation(
            observer + Vec3::Y * player_height,
        ))
        .named("world-law observer")
        .tagged("player");
        let mut player_body = Body::static_body();
        player_body.mode = BodyMode::Kinematic;
        player_body.mass = 72.0;
        player.body = Some(player_body);
        player.collider = Some(Collider::CapsuleY {
            radius: 0.28,
            half_height: 0.55,
        });
        let player = world.spawn(player);
        let mut poc = Self {
            mode,
            ground_y,
            laws: WorldLawRuntime::new(),
            events: Vec::new(),
            eva: EvaState::default(),
            titan: TitanState::default(),
            crossover: CrossoverState::default(),
        };
        match mode {
            LawPocMode::EvaAtField => poc.build_eva_chamber(&mut world),
            LawPocMode::TitanPaths => poc.build_titan_chamber(&mut world),
            LawPocMode::Crossover => poc.build_crossover_chamber(&mut world),
        }
        poc.commit_laws(0);
        (world, player, poc)
    }

    pub fn mode(&self) -> LawPocMode {
        self.mode
    }

    pub fn camera_focus(&self) -> Vec3 {
        match self.mode {
            LawPocMode::EvaAtField => Vec3::new(-0.3, self.ground_y + 2.5, 0.0),
            LawPocMode::TitanPaths => Vec3::new(0.0, self.ground_y + 4.6, 0.0),
            LawPocMode::Crossover => Vec3::new(-0.5, self.ground_y + 2.6, 0.0),
        }
    }

    pub fn camera_distance(&self) -> f32 {
        match self.mode {
            LawPocMode::TitanPaths => 15.5,
            _ => 12.5,
        }
    }

    pub fn title(&self) -> &'static str {
        match self.mode {
            LawPocMode::EvaAtField => "WORLD LAW LAB / A.T. FIELD",
            LawPocMode::TitanPaths => "WORLD LAW LAB / PATHS",
            LawPocMode::Crossover => "WORLD LAW LAB / CROSS-LAW",
        }
    }

    pub fn subtitle(&self) -> String {
        match self.mode {
            LawPocMode::EvaAtField => format!(
                "Soul -> Identity -> Field / blocked {} / penetrated {}",
                self.eva.blocked.len(),
                self.eva.penetrated.len()
            ),
            LawPocMode::TitanPaths => format!(
                "Injury + Intent + Power + Paths / regeneration {:>3.0}%",
                self.regeneration_progress() * 100.0
            ),
            LawPocMode::Crossover => format!(
                "same field law / blocked {} / penetrated {} / special cases 0",
                self.crossover.blocked.len(),
                self.crossover.penetrated.len()
            ),
        }
    }

    pub fn pre_step(&mut self, world: &mut World) {
        let tick = world.tick();
        match self.mode {
            LawPocMode::EvaAtField => self.step_eva_laws(world, tick),
            LawPocMode::TitanPaths => self.step_titan_laws(world, tick),
            LawPocMode::Crossover => self.step_crossover_laws(world, tick),
        }
        self.project_identity_fields(world);
        self.commit_laws(tick.saturating_add(1));
    }

    pub fn post_step(&mut self, world: &mut World, report: &StepReport) {
        if self.mode != LawPocMode::TitanPaths {
            return;
        }
        for event in &report.events {
            let pocket3d_world::WorldEvent::Fractured { entity, .. } = event else {
                continue;
            };
            let is_right_arm = self
                .titan
                .parts
                .iter()
                .any(|part| part.entity == *entity && part.kind == TitanPartKind::RightArm);
            if !is_right_arm || self.titan.severed_limb.is_some() {
                continue;
            }
            if let Some(arm) = world.entity_mut(*entity) {
                arm.attachment = None;
                if let Some(body) = arm.body.as_mut() {
                    body.mode = BodyMode::Dynamic;
                    body.linear_velocity += Vec3::new(1.4, 0.8, 0.5);
                    body.wake();
                }
                self.titan.severed_start_y = Some(arm.transform.position.y);
            }
            self.titan.severed_limb = Some(*entity);
            self.titan.regeneration_started_tick = Some(report.tick.saturating_add(18));
            self.laws.queue_transition(
                "titan.morphology-integrity",
                self.titan.host.expect("Titan host exists"),
                "titan-avatar",
                "limb-missing",
            );
            self.laws.queue_state(
                self.titan.host.expect("Titan host exists"),
                "titan.morphology",
                HiddenValue::Symbol("limb-missing".into()),
            );
        }
    }

    pub fn receipt(&self, world: &World) -> LawPocReceipt {
        let hidden_states = self.laws.states().collect();
        let relations = self.laws.relations().cloned().collect();
        let fields = self.laws.fields().cloned().collect();
        let eva = (self.mode == LawPocMode::EvaAtField).then(|| self.eva_receipt(world));
        let titan = (self.mode == LawPocMode::TitanPaths).then(|| self.titan_receipt(world));
        let crossover = (self.mode == LawPocMode::Crossover).then(|| self.crossover_receipt(world));
        let acceptance_passed = match (&eva, &titan, &crossover) {
            (Some(receipt), _, _) => {
                receipt.ordinary_matter_blocked
                    && receipt.local_interference_observed
                    && receipt.interference_enabled_penetration
            }
            (_, Some(receipt), _) => {
                receipt.trigger_had_injury
                    && receipt.trigger_had_intent
                    && receipt.trigger_had_power
                    && receipt.trigger_had_paths
                    && receipt.avatar_materialized
                    && receipt.severed_limb_fell
                    && receipt.morphology_restored
            }
            (_, _, Some(receipt)) => {
                receipt.low_energy_body_blocked
                    && receipt.high_energy_body_penetrated
                    && receipt.crossover_specific_rule_count == 0
            }
            _ => false,
        };
        LawPocReceipt {
            schema: "pocket3d.world-law.receipt.v1",
            scenario: self.mode.scenario(),
            physics_tick: world.tick(),
            law_tick: self.laws.tick(),
            state_hash: format!("{:016x}", self.laws.state_hash()),
            hidden_states,
            relations,
            fields,
            events: self.events.clone(),
            eva,
            titan,
            crossover,
            acceptance_passed,
        }
    }

    pub fn presentation(&self, world: &World, time: f32) -> PocPresentation {
        let mut output = PocPresentation::default();
        self.render_lab_floor(&mut output);
        match self.mode {
            LawPocMode::EvaAtField => self.render_eva(world, time, &mut output),
            LawPocMode::TitanPaths => self.render_titan(world, time, &mut output),
            LawPocMode::Crossover => self.render_crossover(world, time, &mut output),
        }
        self.render_fields(time, &mut output);
        output
    }

    fn build_eva_chamber(&mut self, world: &mut World) {
        let unit = spawn_static(
            world,
            "evangelion constitution",
            Vec3::new(-1.0, self.ground_y + 3.0, 0.0),
        );
        let soul = spawn_hidden(
            world,
            "soul core",
            Vec3::new(-1.0, self.ground_y + 3.1, 0.0),
        );
        let pilot = spawn_hidden(world, "pilot", Vec3::new(-1.0, self.ground_y + 1.3, 0.45));
        let interferer = spawn_hidden(
            world,
            "opposing identity emitter",
            Vec3::new(3.2, self.ground_y + 3.0, 0.0),
        );
        self.eva.unit = Some(unit);
        self.eva.soul = Some(soul);
        self.eva.pilot = Some(pilot);
        self.eva.interferer = Some(interferer);
        self.laws
            .queue_state(soul, "eva.soul.identity", HiddenValue::Scalar(1.0));
        self.laws
            .queue_state(pilot, "eva.pilot.sync", HiddenValue::Scalar(0.92));
        self.laws
            .queue_state(interferer, "eva.identity.active", HiddenValue::Flag(false));
        self.laws.queue_relation(SYNC_RELATION, pilot, soul, 0.92);
        self.laws.queue_relation(EMBODIED_RELATION, soul, unit, 1.0);
        self.laws.queue_field(SpatialField::new(
            IDENTITY_FIELD,
            unit,
            world.entity(unit).unwrap().transform.position,
            FIELD_RADIUS,
            EVA_FIELD_BUDGET,
            1.0,
        ));
    }

    fn build_titan_chamber(&mut self, world: &mut World) {
        let host = spawn_hidden(
            world,
            "titan power host",
            Vec3::new(-2.8, self.ground_y + 0.9, 0.0),
        );
        let paths = spawn_hidden(
            world,
            "paths nexus",
            Vec3::new(0.0, self.ground_y + 13.0, 0.0),
        );
        self.titan.host = Some(host);
        self.titan.paths = Some(paths);
        self.laws
            .queue_state(host, "titan.subject", HiddenValue::Flag(true));
        self.laws
            .queue_state(host, "titan.power", HiddenValue::Flag(true));
        self.laws
            .queue_state(host, "body.injury", HiddenValue::Scalar(0.0));
        self.laws
            .queue_state(host, "mind.intent", HiddenValue::Scalar(0.0));
        self.laws.queue_state(
            host,
            "titan.morphology",
            HiddenValue::Symbol("human".into()),
        );
        self.laws.queue_relation(PATHS_RELATION, host, paths, 1.0);
    }

    fn build_crossover_chamber(&mut self, world: &mut World) {
        let source = spawn_static(
            world,
            "identity field constitution",
            Vec3::new(-2.1, self.ground_y + 3.0, 0.0),
        );
        let soul = spawn_hidden(
            world,
            "crossover soul core",
            Vec3::new(-2.1, self.ground_y + 3.1, 0.0),
        );
        self.crossover.source = Some(source);
        self.laws
            .queue_state(soul, "eva.soul.identity", HiddenValue::Scalar(1.0));
        self.laws
            .queue_relation(EMBODIED_RELATION, soul, source, 1.0);
        self.laws.queue_field(SpatialField::new(
            IDENTITY_FIELD,
            source,
            world.entity(source).unwrap().transform.position,
            FIELD_RADIUS,
            4_800.0,
            1.0,
        ));
    }

    fn step_eva_laws(&mut self, world: &mut World, tick: u64) {
        let unit = self.eva.unit.expect("EVA unit exists");
        if tick == 8 {
            let projectile = spawn_projectile(
                world,
                "ordinary matter probe",
                Vec3::new(7.0, self.ground_y + 3.0, 0.0),
                Vec3::new(-14.0, 0.0, 0.0),
                4.0,
                Collider::Sphere { radius: 0.22 },
            );
            self.eva.projectiles.push(projectile);
        }
        if tick == 82 {
            let interferer = self.eva.interferer.expect("interferer exists");
            self.laws
                .queue_state(interferer, "eva.identity.active", HiddenValue::Flag(true));
        }
        if tick == 96 {
            let projectile = spawn_projectile(
                world,
                "counterfield matter probe",
                Vec3::new(5.5, self.ground_y + 3.0, 0.8),
                Vec3::new(-14.0, 0.0, 0.0),
                4.0,
                Collider::Sphere { radius: 0.22 },
            );
            self.eva.projectiles.push(projectile);
        }
        let soul = self.eva.soul.expect("soul exists");
        let pilot = self.eva.pilot.expect("pilot exists");
        let identity = self.laws.scalar(soul, "eva.soul.identity").unwrap_or(0.0);
        let sync = self
            .laws
            .relation(SYNC_RELATION, pilot, soul)
            .map_or(0.0, |relation| relation.strength);
        let center = world.entity(unit).unwrap().transform.position;
        self.laws.queue_field(SpatialField::new(
            IDENTITY_FIELD,
            unit,
            center,
            FIELD_RADIUS,
            EVA_FIELD_BUDGET * identity * sync,
            1.0,
        ));
        let interferer = self.eva.interferer.expect("interferer exists");
        if self.laws.flag(interferer, "eva.identity.active") {
            let center = world.entity(interferer).unwrap().transform.position;
            self.laws.queue_field(SpatialField::new(
                IDENTITY_FIELD,
                interferer,
                center,
                3.0,
                EVA_INTERFERENCE_BUDGET,
                -1.0,
            ));
            if self
                .laws
                .resolved_field_at(
                    IDENTITY_FIELD,
                    unit,
                    center.lerp(world.entity(unit).unwrap().transform.position, 0.5),
                )
                .is_some_and(|field| field.cancelled_intensity > 0.0)
            {
                self.eva.interference_observed = true;
            }
        }
        self.update_penetration_receipts(world, unit, &self.eva.projectiles.clone());
    }

    fn step_titan_laws(&mut self, world: &mut World, tick: u64) {
        let host = self.titan.host.expect("Titan host exists");
        let paths = self.titan.paths.expect("Paths exists");
        if tick == 12 {
            self.laws
                .queue_state(host, "body.injury", HiddenValue::Scalar(1.0));
        }
        if tick == 30 {
            self.laws
                .queue_state(host, "mind.intent", HiddenValue::Scalar(1.0));
        }
        let injury = self.laws.scalar(host, "body.injury").unwrap_or(0.0) >= 1.0;
        let intent = self.laws.scalar(host, "mind.intent").unwrap_or(0.0) >= 1.0;
        let power = self.laws.flag(host, "titan.power");
        let paths_connected = self
            .laws
            .relation(PATHS_RELATION, host, paths)
            .is_some_and(|relation| relation.strength > 0.0);
        if injury && intent && power && paths_connected && self.titan.torso.is_none() {
            self.titan.trigger_had_injury = true;
            self.titan.trigger_had_intent = true;
            self.titan.trigger_had_power = true;
            self.titan.trigger_had_paths = true;
            self.materialize_titan_avatar(world, tick);
        }
        self.advance_avatar_materialization(world, tick);
        if tick == 156
            && let Some(right_arm) = self
                .titan
                .parts
                .iter()
                .find(|part| part.kind == TitanPartKind::RightArm && !part.regeneration)
                .map(|part| part.entity)
        {
            world.queue_interaction(Interaction::Cut {
                target: right_arm,
                direction: Vec3::new(1.0, 0.2, 0.4).normalize(),
                energy: 2.0,
            });
        }
        self.advance_regeneration(world, tick);
    }

    fn step_crossover_laws(&mut self, world: &mut World, tick: u64) {
        let source = self.crossover.source.expect("field source exists");
        let center = world.entity(source).unwrap().transform.position;
        self.laws.queue_field(SpatialField::new(
            IDENTITY_FIELD,
            source,
            center,
            FIELD_RADIUS,
            4_800.0,
            1.0,
        ));
        if tick == 8 {
            let body = spawn_projectile(
                world,
                "ordinary titan biomass impact",
                Vec3::new(6.5, self.ground_y + 3.0, -0.75),
                Vec3::new(-11.0, 0.0, 0.0),
                52.0,
                Collider::CapsuleY {
                    radius: 0.54,
                    half_height: 0.72,
                },
            );
            self.crossover.blocked_body = Some(body);
            self.crossover
                .initial_energy
                .insert(body, kinetic_energy(world, body));
        }
        if tick == 82 {
            let body = spawn_projectile(
                world,
                "high momentum titan biomass impact",
                Vec3::new(6.5, self.ground_y + 3.0, 0.85),
                Vec3::new(-16.0, 0.0, 0.0),
                52.0,
                Collider::CapsuleY {
                    radius: 0.54,
                    half_height: 0.72,
                },
            );
            self.crossover.penetrating_body = Some(body);
            self.crossover
                .initial_energy
                .insert(body, kinetic_energy(world, body));
        }
        let bodies: Vec<_> = [self.crossover.blocked_body, self.crossover.penetrating_body]
            .into_iter()
            .flatten()
            .collect();
        for body in bodies {
            if let Some(entity) = world.entity(body) {
                let speed = entity.body.map_or(0.0, |body| body.linear_velocity.x.abs());
                if entity.transform.position.x > center.x + FIELD_RADIUS - 0.2 && speed < 0.25 {
                    self.crossover.blocked.insert(body);
                }
                if entity.transform.position.x < center.x - 0.5 {
                    self.crossover.penetrated.insert(body);
                }
            }
        }
    }

    fn project_identity_fields(&mut self, world: &mut World) {
        let fields: Vec<_> = self
            .laws
            .fields()
            .filter(|field| field.key.channel == IDENTITY_FIELD)
            .cloned()
            .collect();
        let bodies: Vec<_> = world
            .entities()
            .filter_map(|(&id, entity)| {
                let body = entity.body?;
                (body.mode == BodyMode::Dynamic && entity.attachment.is_none()).then_some((
                    id,
                    entity.transform,
                    entity.collider,
                    body,
                ))
            })
            .collect();
        for field in fields {
            for (id, transform, collider, body) in &bodies {
                let bound_radius = collider
                    .map_or(0.05, |collider| collider.radius() + collider.half_height())
                    * transform.scale.abs().max_element();
                let contact_point = field.center
                    + (transform.position - field.center).normalize_or(Vec3::X) * field.radius;
                let Some(resolved) = self.laws.resolved_field_at(
                    &field.key.channel,
                    field.key.source,
                    contact_point,
                ) else {
                    continue;
                };
                let probe = FieldBodyProbe {
                    position: transform.position,
                    velocity: body.linear_velocity,
                    mass: body.mass,
                    bound_radius,
                };
                let Some(projection) = spherical_barrier_projection(
                    field.center,
                    field.radius,
                    resolved.effective_intensity,
                    probe,
                    world.config().fixed_dt,
                ) else {
                    continue;
                };
                world.queue_interaction(Interaction::Impulse {
                    target: *id,
                    impulse: projection.impulse,
                    point: projection.contact_point,
                });
                self.laws.queue_projection(LawProjection {
                    law: "eva.identity-boundary-projection".into(),
                    source: field.key.source,
                    target: Some(*id),
                    kind: if projection.blocked {
                        "kinetic-block".into()
                    } else {
                        "kinetic-attenuation".into()
                    },
                    magnitude: projection.absorbed_energy,
                    position: projection.contact_point,
                    direction: projection.impulse,
                });
                match self.mode {
                    LawPocMode::EvaAtField => {
                        if projection.blocked {
                            self.eva.blocked.insert(*id);
                        }
                    }
                    LawPocMode::Crossover => {
                        if projection.blocked {
                            self.crossover.blocked.insert(*id);
                        }
                    }
                    LawPocMode::TitanPaths => {}
                }
            }
        }
    }

    fn materialize_titan_avatar(&mut self, world: &mut World, tick: u64) {
        let host = self.titan.host.expect("Titan host exists");
        let torso_position = Vec3::new(0.0, self.ground_y + 5.0, 0.0);
        let mut torso = EntityBundle::new(Transform::from_translation(torso_position))
            .named("titan avatar torso")
            .tagged("titan-avatar")
            .tagged("biomass");
        let mut body = Body::static_body();
        body.mode = BodyMode::Kinematic;
        body.mass = 420.0;
        torso.body = Some(body);
        torso.collider = Some(Collider::CapsuleY {
            radius: 1.08,
            half_height: 1.8,
        });
        let torso = world.spawn(torso);
        self.titan.torso = Some(torso);
        self.titan.avatar_started_tick = Some(tick);
        self.titan.parts.push(TitanPart {
            entity: torso,
            kind: TitanPartKind::Torso,
            materialization: 0.05,
            regeneration: false,
        });
        for kind in [
            TitanPartKind::Head,
            TitanPartKind::LeftLeg,
            TitanPartKind::RightLeg,
            TitanPartKind::LeftArm,
            TitanPartKind::RightArm,
        ] {
            let entity = spawn_titan_part(world, torso, kind, false);
            self.titan.parts.push(TitanPart {
                entity,
                kind,
                materialization: 0.0,
                regeneration: false,
            });
        }
        self.laws.queue_transition(
            "titan.materialization",
            host,
            "human",
            "titan-materializing",
        );
        self.laws.queue_state(
            host,
            "titan.morphology",
            HiddenValue::Symbol("titan-materializing".into()),
        );
    }

    fn advance_avatar_materialization(&mut self, _world: &mut World, tick: u64) {
        let Some(start) = self.titan.avatar_started_tick else {
            return;
        };
        let host = self.titan.host.expect("Titan host exists");
        let progress = ((tick.saturating_sub(start)) as f32 / 72.0).clamp(0.0, 1.0);
        for (index, part) in self.titan.parts.iter_mut().enumerate() {
            if part.regeneration {
                continue;
            }
            let stagger = index as f32 * 0.07;
            let previous = part.materialization;
            part.materialization =
                ((progress - stagger) / (1.0 - stagger).max(0.01)).clamp(0.0, 1.0);
            if crossed_milestone(previous, part.materialization) {
                self.laws.queue_materialization(
                    "titan.paths-biological-materialization",
                    host,
                    part.entity,
                    titan_part_name(part.kind),
                    part.materialization,
                );
            }
        }
        if progress >= 1.0 && self.laws.symbol(host, "titan.morphology") != Some("titan-avatar") {
            self.laws.queue_transition(
                "titan.materialization",
                host,
                "titan-materializing",
                "titan-avatar",
            );
            self.laws.queue_state(
                host,
                "titan.morphology",
                HiddenValue::Symbol("titan-avatar".into()),
            );
        }
    }

    fn advance_regeneration(&mut self, world: &mut World, tick: u64) {
        let Some(start) = self.titan.regeneration_started_tick else {
            return;
        };
        let torso = self.titan.torso.expect("Titan torso exists");
        let host = self.titan.host.expect("Titan host exists");
        if tick >= start && self.titan.regenerated_limb.is_none() {
            let arm = spawn_titan_part(world, torso, TitanPartKind::RightArm, true);
            self.titan.regenerated_limb = Some(arm);
            self.titan.parts.push(TitanPart {
                entity: arm,
                kind: TitanPartKind::RightArm,
                materialization: 0.02,
                regeneration: true,
            });
        }
        let progress =
            ((tick.saturating_sub(start)) as f32 / TITAN_REGEN_TURNS as f32).clamp(0.0, 1.0);
        if let Some(arm) = self.titan.regenerated_limb
            && let Some(part) = self.titan.parts.iter_mut().find(|part| part.entity == arm)
        {
            let previous = part.materialization;
            part.materialization = progress.max(0.02);
            if crossed_milestone(previous, part.materialization) {
                self.laws.queue_materialization(
                    "titan.paths-regeneration",
                    host,
                    arm,
                    "right-arm",
                    part.materialization,
                );
            }
        }
        if progress >= 1.0 && self.laws.symbol(host, "titan.morphology") != Some("titan-avatar") {
            self.laws.queue_transition(
                "titan.morphology-integrity",
                host,
                "limb-missing",
                "titan-avatar",
            );
            self.laws.queue_state(
                host,
                "titan.morphology",
                HiddenValue::Symbol("titan-avatar".into()),
            );
        }
    }

    fn update_penetration_receipts(
        &mut self,
        world: &World,
        source: EntityId,
        projectiles: &[EntityId],
    ) {
        let center = world.entity(source).unwrap().transform.position;
        for projectile in projectiles {
            if world
                .entity(*projectile)
                .is_some_and(|entity| entity.transform.position.x < center.x - 0.5)
            {
                self.eva.penetrated.insert(*projectile);
            }
        }
    }

    fn commit_laws(&mut self, physics_tick: u64) {
        let report = self.laws.step();
        for event in report.events {
            if self.events.len() < 192 {
                self.events.push(TimedLawEvent {
                    tick: physics_tick,
                    event,
                });
            }
        }
    }

    fn regeneration_progress(&self) -> f32 {
        self.titan
            .regenerated_limb
            .and_then(|arm| {
                self.titan
                    .parts
                    .iter()
                    .find(|part| part.entity == arm)
                    .map(|part| part.materialization)
            })
            .unwrap_or(0.0)
    }

    fn eva_receipt(&self, _world: &World) -> EvaLawReceipt {
        let pilot = self.eva.pilot.expect("pilot exists");
        let soul = self.eva.soul.expect("soul exists");
        let unit = self.eva.unit.expect("unit exists");
        let sync = self
            .laws
            .relation(SYNC_RELATION, pilot, soul)
            .map_or(0.0, |relation| relation.strength);
        EvaLawReceipt {
            pilot: pilot.0,
            soul: soul.0,
            unit: unit.0,
            synchronization: sync,
            field_budget: EVA_FIELD_BUDGET * sync,
            interference_budget: EVA_INTERFERENCE_BUDGET,
            blocked_projectiles: self.eva.blocked.iter().map(|id| id.0).collect(),
            penetrated_projectiles: self.eva.penetrated.iter().map(|id| id.0).collect(),
            ordinary_matter_blocked: self
                .eva
                .projectiles
                .first()
                .is_some_and(|id| self.eva.blocked.contains(id)),
            local_interference_observed: self.eva.interference_observed,
            interference_enabled_penetration: self
                .eva
                .projectiles
                .get(1)
                .is_some_and(|id| self.eva.penetrated.contains(id)),
        }
    }

    fn titan_receipt(&self, world: &World) -> TitanLawReceipt {
        let severed_limb_fell = self
            .titan
            .severed_limb
            .zip(self.titan.severed_start_y)
            .is_some_and(|(limb, start_y)| {
                world
                    .entity(limb)
                    .is_some_and(|entity| entity.transform.position.y < start_y - 1.0)
            });
        let progress = self.regeneration_progress();
        let host = self.titan.host.expect("Titan host exists");
        TitanLawReceipt {
            host: host.0,
            paths: self.titan.paths.expect("Paths exists").0,
            trigger_had_injury: self.titan.trigger_had_injury,
            trigger_had_intent: self.titan.trigger_had_intent,
            trigger_had_power: self.titan.trigger_had_power,
            trigger_had_paths: self.titan.trigger_had_paths,
            avatar_materialized: self
                .titan
                .parts
                .iter()
                .all(|part| part.regeneration || part.materialization >= 1.0),
            severed_limb: self.titan.severed_limb.map(|id| id.0),
            severed_limb_fell,
            regenerated_limb: self.titan.regenerated_limb.map(|id| id.0),
            regeneration_progress: progress,
            morphology_restored: progress >= 1.0
                && self.laws.symbol(host, "titan.morphology") == Some("titan-avatar"),
        }
    }

    fn crossover_receipt(&self, world: &World) -> CrossoverLawReceipt {
        let blocked = self.crossover.blocked_body.expect("blocked body exists");
        let penetrating = self
            .crossover
            .penetrating_body
            .expect("penetrating body exists");
        let source = self.crossover.source.expect("source exists");
        let center = world.entity(source).unwrap().transform.position;
        let low_energy_body_blocked = self.crossover.blocked.contains(&blocked)
            && world.entity(blocked).is_some_and(|entity| {
                entity.transform.position.x > center.x + FIELD_RADIUS - 0.3
                    && entity
                        .body
                        .is_some_and(|body| body.linear_velocity.x.abs() < 0.3)
            });
        CrossoverLawReceipt {
            field_source: source.0,
            blocked_body: blocked.0,
            penetrating_body: penetrating.0,
            blocked_body_energy: self.crossover.initial_energy[&blocked],
            penetrating_body_energy: self.crossover.initial_energy[&penetrating],
            field_budget: 4_800.0,
            low_energy_body_blocked,
            high_energy_body_penetrated: self.crossover.penetrated.contains(&penetrating),
            crossover_specific_rule_count: 0,
        }
    }

    fn render_lab_floor(&self, output: &mut PocPresentation) {
        output.models.push(PocModel {
            kind: PocModelKind::Disc,
            transform: Mat4::from_scale_rotation_translation(
                Vec3::new(8.5, 0.03, 8.5),
                Quat::IDENTITY,
                Vec3::new(0.0, self.ground_y + 0.025, 0.0),
            ),
            tint: [0.06, 0.075, 0.085, 1.0],
            lit: true,
        });
        for radius in [3.0_f32, 5.5, 8.0] {
            push_ring(
                &mut output.beams,
                Vec3::new(0.0, self.ground_y + 0.06, 0.0),
                radius,
                32,
                0.025,
                [0.12, 0.35, 0.42, 0.48],
            );
        }
    }

    fn render_eva(&self, world: &World, time: f32, output: &mut PocPresentation) {
        let Some(unit) = self.eva.unit.and_then(|id| world.entity(id)) else {
            return;
        };
        push_eva_unit(output, unit.transform.position, time);
        if let Some(pilot) = self.eva.pilot.and_then(|id| world.entity(id)) {
            push_human(output, pilot.transform.position, [0.88, 0.84, 0.72, 1.0]);
        }
        if let Some(interferer) = self.eva.interferer.and_then(|id| world.entity(id)) {
            output.models.push(PocModel {
                kind: PocModelKind::Sphere,
                transform: Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.38),
                    Quat::IDENTITY,
                    interferer.transform.position,
                ),
                tint: if self.laws.flag(interferer.id, "eva.identity.active") {
                    [1.0, 0.18, 0.08, 1.0]
                } else {
                    [0.26, 0.12, 0.10, 1.0]
                },
                lit: false,
            });
        }
        self.render_projectiles(world, &self.eva.projectiles, output);
    }

    fn render_titan(&self, world: &World, time: f32, output: &mut PocPresentation) {
        let host = self.titan.host.and_then(|id| world.entity(id));
        if let Some(host) = host {
            push_human(output, host.transform.position, [0.30, 0.34, 0.38, 1.0]);
        }
        let body_progress = self
            .titan
            .parts
            .iter()
            .find(|part| part.kind == TitanPartKind::Torso)
            .map_or(0.0, |part| part.materialization);
        let body_grow = (0.16 + body_progress * 0.84).clamp(0.0, 1.0);
        if body_progress > 0.0 {
            output.models.push(PocModel {
                kind: PocModelKind::ColossalTitanBody,
                transform: Mat4::from_scale_rotation_translation(
                    Vec3::splat(body_grow),
                    Quat::IDENTITY,
                    Vec3::new(0.0, self.ground_y, 0.0),
                ),
                tint: titan_materialization_tint(body_progress),
                lit: true,
            });
        }

        let simulated_shoulder = Vec3::new(1.65, self.ground_y + 5.45, 0.0);
        let authored_shoulder = Vec3::new(
            1.79075 * body_grow,
            self.ground_y + 8.54791 * body_grow,
            -0.36712 * body_grow,
        );
        for part in self
            .titan
            .parts
            .iter()
            .filter(|part| part.kind == TitanPartKind::RightArm)
        {
            let Some(entity) = world.entity(part.entity) else {
                continue;
            };
            let progress = part.materialization;
            if progress <= 0.0 {
                continue;
            }
            let grow = (0.22 + progress * 0.78).clamp(0.0, 1.0);
            output.models.push(PocModel {
                kind: PocModelKind::ColossalTitanRightArm,
                transform: Mat4::from_scale_rotation_translation(
                    Vec3::splat(grow),
                    entity.transform.rotation * Quat::from_rotation_z(-0.18),
                    authored_shoulder + entity.transform.position - simulated_shoulder,
                ),
                tint: titan_materialization_tint(progress),
                lit: true,
            });
            let residual_steam = self
                .titan
                .regeneration_started_tick
                .is_some_and(|start| world.tick() <= start + TITAN_REGEN_TURNS + 60);
            if part.regeneration && (progress < 1.0 || residual_steam) {
                for index in 0..9 {
                    let phase = time * 1.7 + index as f32 * 2.13;
                    output.sprites.push(PocSprite {
                        position: entity.transform.position
                            + Vec3::new(
                                phase.sin() * 0.65,
                                index as f32 * 0.30 - 1.0,
                                phase.cos() * 0.65,
                            ),
                        size: 0.32 + index as f32 * 0.025,
                        color: [0.88, 0.94, 0.90, 0.42],
                    });
                }
            }
        }
        let Some(paths) = self.titan.paths.and_then(|id| world.entity(id)) else {
            return;
        };
        let target = self
            .titan
            .regenerated_limb
            .and_then(|id| world.entity(id))
            .or_else(|| self.titan.torso.and_then(|id| world.entity(id)));
        if let Some(target) = target {
            for offset in [-0.55_f32, 0.0, 0.55] {
                output.beams.push(PocBeam {
                    a: paths.transform.position + Vec3::X * offset,
                    b: target.transform.position + Vec3::X * offset * 0.25,
                    width: 0.095,
                    color: [1.0, 0.38, 0.035, 0.92],
                });
            }
            for index in 0..9 {
                let fraction = index as f32 / 8.0;
                output.sprites.push(PocSprite {
                    position: paths
                        .transform
                        .position
                        .lerp(target.transform.position, fraction)
                        + Vec3::new((index as f32 * 2.4).sin() * 0.28, 0.0, 0.0),
                    size: 0.22,
                    color: [1.0, 0.52, 0.06, 0.82],
                });
            }
        }
    }

    fn render_crossover(&self, world: &World, time: f32, output: &mut PocPresentation) {
        if let Some(source) = self.crossover.source.and_then(|id| world.entity(id)) {
            push_eva_unit(output, source.transform.position, time);
        }
        let bodies: Vec<_> = [self.crossover.blocked_body, self.crossover.penetrating_body]
            .into_iter()
            .flatten()
            .collect();
        for (index, body) in bodies.iter().enumerate() {
            if let Some(entity) = world.entity(*body) {
                output.models.push(PocModel {
                    kind: PocModelKind::Sphere,
                    transform: Mat4::from_scale_rotation_translation(
                        Vec3::new(0.92, 0.68, 0.78),
                        entity.transform.rotation,
                        entity.transform.position,
                    ),
                    tint: if index == 0 {
                        [0.76, 0.26, 0.20, 1.0]
                    } else {
                        [0.98, 0.40, 0.16, 1.0]
                    },
                    lit: true,
                });
                output.models.push(PocModel {
                    kind: PocModelKind::Cylinder,
                    transform: Mat4::from_scale_rotation_translation(
                        Vec3::new(0.52, 2.4, 0.52),
                        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                        entity.transform.position + Vec3::X * 2.0,
                    ),
                    tint: [0.56, 0.09, 0.06, 1.0],
                    lit: true,
                });
            }
        }
    }

    fn render_projectiles(
        &self,
        world: &World,
        projectiles: &[EntityId],
        output: &mut PocPresentation,
    ) {
        for (index, projectile) in projectiles.iter().enumerate() {
            let Some(entity) = world.entity(*projectile) else {
                continue;
            };
            output.models.push(PocModel {
                kind: PocModelKind::Sphere,
                transform: Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.28),
                    entity.transform.rotation,
                    entity.transform.position,
                ),
                tint: if index == 0 {
                    [1.0, 0.65, 0.08, 1.0]
                } else {
                    [1.0, 0.16, 0.08, 1.0]
                },
                lit: false,
            });
        }
    }

    fn render_fields(&self, time: f32, output: &mut PocPresentation) {
        for field in self.laws.fields() {
            let color = if field.polarity >= 0.0 {
                [0.66, 0.08, 1.0, 0.82]
            } else {
                [1.0, 0.10, 0.025, 0.78]
            };
            for latitude in [-0.48_f32, 0.0, 0.48] {
                let ring_radius = field.radius * (1.0 - latitude * latitude).sqrt();
                let center = field.center + Vec3::Y * field.radius * latitude;
                push_ring(&mut output.beams, center, ring_radius, 40, 0.055, color);
            }
            push_oriented_ring(
                &mut output.beams,
                field.center,
                Vec3::X,
                Vec3::Y,
                field.radius,
                40,
                0.062,
                color,
            );
            push_oriented_ring(
                &mut output.beams,
                field.center,
                Vec3::Y,
                Vec3::Z,
                field.radius,
                40,
                0.062,
                color,
            );
            for index in 0..10 {
                let phase = time * 0.35 + index as f32 / 10.0 * std::f32::consts::TAU;
                let direction = Vec3::new(phase.cos(), (phase * 1.7).sin() * 0.42, phase.sin())
                    .normalize_or(Vec3::X);
                output.sprites.push(PocSprite {
                    position: field.center + direction * field.radius,
                    size: 0.17,
                    color,
                });
            }
        }
    }
}

fn spawn_hidden(world: &mut World, name: &str, position: Vec3) -> EntityId {
    world.spawn(EntityBundle::new(Transform::from_translation(position)).named(name))
}

fn spawn_static(world: &mut World, name: &str, position: Vec3) -> EntityId {
    let mut entity = EntityBundle::new(Transform::from_translation(position)).named(name);
    entity.body = Some(Body::static_body());
    world.spawn(entity)
}

fn spawn_projectile(
    world: &mut World,
    name: &str,
    position: Vec3,
    velocity: Vec3,
    mass: f32,
    collider: Collider,
) -> EntityId {
    let mut entity = EntityBundle::new(Transform::from_translation(position))
        .named(name)
        .tagged("law-probe");
    let mut body = Body::dynamic(mass);
    body.gravity_scale = 0.0;
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;
    body.linear_velocity = velocity;
    entity.body = Some(body);
    entity.collider = Some(collider);
    entity.surface = PhysicalSurface {
        friction: 0.0,
        restitution: 0.0,
    };
    world.spawn(entity)
}

fn spawn_titan_part(
    world: &mut World,
    torso: EntityId,
    kind: TitanPartKind,
    regeneration: bool,
) -> EntityId {
    let (local, collider, mass) = match kind {
        TitanPartKind::Head => (
            Transform::from_translation(Vec3::new(0.0, 3.0, 0.0)),
            Collider::Sphere { radius: 0.78 },
            48.0,
        ),
        TitanPartKind::LeftArm => (
            Transform {
                position: Vec3::new(-1.65, 0.45, 0.0),
                rotation: Quat::from_rotation_z(-0.18),
                scale: Vec3::ONE,
            },
            Collider::CapsuleY {
                radius: 0.42,
                half_height: 1.45,
            },
            62.0,
        ),
        TitanPartKind::RightArm => (
            Transform {
                position: Vec3::new(1.65, 0.45, 0.0),
                rotation: Quat::from_rotation_z(0.18),
                scale: Vec3::ONE,
            },
            Collider::CapsuleY {
                radius: 0.42,
                half_height: 1.45,
            },
            62.0,
        ),
        TitanPartKind::LeftLeg => (
            Transform::from_translation(Vec3::new(-0.58, -3.25, 0.0)),
            Collider::CapsuleY {
                radius: 0.50,
                half_height: 1.55,
            },
            105.0,
        ),
        TitanPartKind::RightLeg => (
            Transform::from_translation(Vec3::new(0.58, -3.25, 0.0)),
            Collider::CapsuleY {
                radius: 0.50,
                half_height: 1.55,
            },
            105.0,
        ),
        TitanPartKind::Torso => unreachable!("torso is the morphology root"),
    };
    let parent = world.entity(torso).expect("torso exists").transform;
    let mut part = EntityBundle::new(parent.compose(local))
        .named(if regeneration {
            "regenerated titan right arm"
        } else {
            titan_part_name(kind)
        })
        .tagged("titan-avatar")
        .tagged("biomass");
    part.body = Some(Body::dynamic(mass));
    part.collider = Some(collider);
    part.attachment = Some(Attachment {
        parent: torso,
        local,
        release_impulse: Vec3::ZERO,
    });
    if kind == TitanPartKind::RightArm && !regeneration {
        part.structure = Some(Structure::new(1.0, 1.0));
    }
    world.spawn(part)
}

fn titan_part_name(kind: TitanPartKind) -> &'static str {
    match kind {
        TitanPartKind::Torso => "torso",
        TitanPartKind::Head => "head",
        TitanPartKind::LeftArm => "left-arm",
        TitanPartKind::RightArm => "right-arm",
        TitanPartKind::LeftLeg => "left-leg",
        TitanPartKind::RightLeg => "right-leg",
    }
}

fn titan_materialization_tint(progress: f32) -> [f32; 4] {
    let progress = progress.clamp(0.0, 1.0);
    [
        0.68 + progress * 0.32,
        0.50 + progress * 0.50,
        0.46 + progress * 0.54,
        1.0,
    ]
}

fn kinetic_energy(world: &World, entity: EntityId) -> f32 {
    world
        .entity(entity)
        .and_then(|entity| entity.body)
        .map_or(0.0, |body| {
            0.5 * body.mass * body.linear_velocity.length_squared()
        })
}

fn crossed_milestone(previous: f32, current: f32) -> bool {
    [0.05_f32, 0.34, 0.72, 1.0]
        .into_iter()
        .any(|milestone| previous < milestone && current >= milestone)
}

fn push_eva_unit(output: &mut PocPresentation, center: Vec3, time: f32) {
    output.models.push(PocModel {
        kind: PocModelKind::EvaUnit01,
        transform: Mat4::from_translation(center - Vec3::Y * 3.0),
        tint: [1.0, 1.0, 1.0, 1.0],
        lit: true,
    });
    output.sprites.push(PocSprite {
        position: center + Vec3::new(0.0, 0.56, 0.72),
        size: 0.12 + (time * 2.0).sin() * 0.015,
        color: [1.0, 0.34, 0.05, 0.84],
    });
}

fn push_human(output: &mut PocPresentation, feet: Vec3, tint: [f32; 4]) {
    output.models.push(PocModel {
        kind: PocModelKind::Cylinder,
        transform: Mat4::from_scale_rotation_translation(
            Vec3::new(0.16, 0.82, 0.16),
            Quat::IDENTITY,
            feet + Vec3::Y * 0.82,
        ),
        tint,
        lit: true,
    });
    output.models.push(PocModel {
        kind: PocModelKind::Sphere,
        transform: Mat4::from_scale_rotation_translation(
            Vec3::splat(0.22),
            Quat::IDENTITY,
            feet + Vec3::Y * 1.82,
        ),
        tint,
        lit: true,
    });
}

fn push_ring(
    beams: &mut Vec<PocBeam>,
    center: Vec3,
    radius: f32,
    segments: usize,
    width: f32,
    color: [f32; 4],
) {
    for index in 0..segments {
        let a = index as f32 / segments as f32 * std::f32::consts::TAU;
        let b = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        beams.push(PocBeam {
            a: center + Vec3::new(a.cos() * radius, 0.0, a.sin() * radius),
            b: center + Vec3::new(b.cos() * radius, 0.0, b.sin() * radius),
            width,
            color,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn push_oriented_ring(
    beams: &mut Vec<PocBeam>,
    center: Vec3,
    axis_a: Vec3,
    axis_b: Vec3,
    radius: f32,
    segments: usize,
    width: f32,
    color: [f32; 4],
) {
    for index in 0..segments {
        let a = index as f32 / segments as f32 * std::f32::consts::TAU;
        let b = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        beams.push(PocBeam {
            a: center + axis_a * (a.cos() * radius) + axis_b * (a.sin() * radius),
            b: center + axis_a * (b.cos() * radius) + axis_b * (b.sin() * radius),
            width,
            color,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocket3d_world::{EnvironmentSample, FlatEnvironment};

    fn run(mode: LawPocMode, turns: u64) -> (World, LawPoc) {
        let (mut world, _, mut poc) = LawPoc::build_world(mode, 7, 0.0, 0.84);
        let environment = FlatEnvironment {
            sample: EnvironmentSample {
                ground_height: 0.0,
                ..EnvironmentSample::default()
            },
        };
        for _ in 0..turns {
            poc.pre_step(&mut world);
            let report = world.step(&environment);
            poc.post_step(&mut world, &report);
        }
        (world, poc)
    }

    #[test]
    fn eva_laws_block_then_allow_penetration_through_local_interference() {
        let (world, poc) = run(LawPocMode::EvaAtField, 190);
        let receipt = poc.receipt(&world);
        assert!(receipt.acceptance_passed, "{receipt:#?}");
        let eva = receipt.eva.unwrap();
        assert!(eva.ordinary_matter_blocked);
        assert!(eva.local_interference_observed);
        assert!(eva.interference_enabled_penetration);
    }

    #[test]
    fn paths_trigger_materializes_and_restores_a_severed_limb() {
        let (world, poc) = run(LawPocMode::TitanPaths, 300);
        let receipt = poc.receipt(&world);
        assert!(receipt.acceptance_passed, "{receipt:#?}");
        let titan = receipt.titan.unwrap();
        assert!(titan.avatar_materialized);
        assert!(titan.severed_limb_fell);
        assert!(titan.morphology_restored);
    }

    #[test]
    fn crossover_uses_energy_and_field_budget_without_a_pair_specific_rule() {
        let (world, poc) = run(LawPocMode::Crossover, 190);
        let receipt = poc.receipt(&world);
        assert!(receipt.acceptance_passed, "{receipt:#?}");
        let crossover = receipt.crossover.unwrap();
        assert!(crossover.blocked_body_energy < crossover.field_budget);
        assert!(crossover.penetrating_body_energy > crossover.field_budget);
        assert_eq!(crossover.crossover_specific_rule_count, 0);
    }

    #[test]
    fn law_scenarios_are_deterministic() {
        for mode in [
            LawPocMode::EvaAtField,
            LawPocMode::TitanPaths,
            LawPocMode::Crossover,
        ] {
            let (a_world, a) = run(mode, 190);
            let (b_world, b) = run(mode, 190);
            assert_eq!(
                serde_json::to_value(a.receipt(&a_world)).unwrap(),
                serde_json::to_value(b.receipt(&b_world)).unwrap()
            );
        }
    }
}
