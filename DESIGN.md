# Pocket Openworld runtime design

## Scope

The application proves that Pocket3D can host a small world whose objects share
the same physical and reactive rules. It includes a controllable explorer,
apple trees, detachable fruit, an axe, rigid bodies, heat, moisture,
combustion, cooking, and fire propagation.

**`pocket3d-world` owns simulation state and has no GPU, window, or asset
dependency.** This repository owns the orchard recipe, input mapping,
procedural art, camera, particles, and HUD. `pocket3d` owns rendering and
application hosting. An object can therefore be simulated headlessly,
rendered by another backend, or configured by a future guest without moving
authoritative state out of the Rust runtime.

## Runtime data

Every object has a stable `EntityId` and a transform. Optional records add
behavior:

- `Body` and `Collider` determine motion and contacts.
- `PhysicalSurface` contains friction and restitution.
- `Attachment` keeps fruit or tools relative to a parent until released.
- `Structure` accumulates directed damage and emits a fracture event.
- `ReactiveMaterial` contains heat capacity, conductivity, ignition, and
  burn-rate parameters.
- `ReactiveState` contains temperature, moisture, fuel, and cooking progress.

**Physical surface properties and reactive material properties are separate.**
A wet apple can have the same bounce as a dry apple while requiring more heat
to ignite. A stone can conduct heat without holding fuel. The renderer reads
the resulting state but does not decide whether an object burns or breaks.

**Heat and retained water use one energy budget.** `Douse` adds normalized
liquid-water mass at `WorldConfig::water_inlet_temperature_c`; the dry
material's `heat_capacity` and the water contribution from
`water_specific_heat` determine the energy-conserving mixed temperature.
Evaporation begins above `water_boiling_temperature_c` and removes both the
water's sensible heat and `water_vaporization_heat` for each unit evaporated,
so evaporation cannot exceed the available thermal energy. A material's
`heat_output` is combustion energy per unit of fuel: each turn releases only
`fuel_consumed * heat_output`, retains
`combustion_local_heat_fraction` in the source, and distributes at most the
remaining budget among nearby receivers. Adding receivers divides that budget;
it does not duplicate heat.

## Fixed update

One simulation turn uses this order:

1. Resolve attachments from parent transforms.
2. Consume queued player interactions and spatial water packets, then rainfall.
3. Sample ambient temperature, moisture, wind, and ground height.
4. Transfer heat, evaporate moisture, ignite eligible fuel, and consume fuel.
5. Integrate dynamic bodies and resolve ground and body contacts.
6. Commit fractures and attachment releases.
7. Publish phase-ordered contact and state-transition events.

**The entity store is traversed by stable ID, structural changes are deferred,
and random values come from a seeded generator.** A seed plus an interaction
script produces the same state hash and ordered event sequence.

Physics callbacks do not directly run game behavior. They produce contact
records containing both IDs, position, normal, impulse, and both physical
surfaces. Structure and reaction rules consume these records in their assigned
phase. This prevents a contact insertion order from changing the result.

**Gameplay fixes must preserve the shared simulation rules.** Collision,
integration, attachment, structure, and reaction solvers must not branch on an
entity ID, tag, recipe, or scenario. Change the shared rule, state the invariant
that the change preserves, and test that invariant across at least two entity
configurations, material combinations, or collider combinations. A scenario
regression test is additional coverage, not the proof of the shared rule.

## Orchard composition

The orchard uses the generic records without tree-specific branches in the
runtime:

**The wood, fruit, and flame coefficient sets are application recipes.** They
live beside the orchard composition in this repository; `pocket3d-world` owns
the material record and reaction solver without naming those content types.

- A standing tree has a static capsule body, fuel, and a `Structure` record.
- Apples have sphere bodies attached at authored local offsets.
- Axe contact queues directed structural damage and an impulse.
- Fracture changes the tree body to dynamic and releases its attached apples.
- The stump is presentation geometry retained at the fracture position.
- Fire is a hot reactive entity with a spatial collider, fuel, and a short
  visual particle trail.
- Heat cooks an apple before sustained heat chars it. Moisture absorbs heat
  and evaporates before dry fuel can ignite.

The fallen trunk remains a normal body and reactive object. It can roll,
collide, heat nearby fruit, propagate fire, exhaust its fuel, and cool down.

## Presentation

Environment meshes are generated from mathematical primitives in this
repository. The scene uses reusable assets with per-instance transforms and
tints for terrain, trunk, canopy, apple, rock, grass, and shadow discs. Fire
and embers use the additive sprite pass.

**The active local character uses Pocket3D's normal glTF skin and clip path.**
The Frieren importer preserves the downloaded model's 61-joint rig, authored
texture, hair, hands, and staff, adds a `staff.tip` socket, then authors `Idle`,
`Walk`, `Chop`, `Cast`, and `Water` actions. The self-contained GLB is embedded
in the executable and loaded through `ModelAsset`; the application does not
define a second animation renderer.

The simulation places the player capsule center at `ground + PLAYER_HEIGHT`.
The presentation transform separately maps the model's authored rest-pose
minimum Y to the sampled terrain height. **The collision origin stays at the
capsule center while the rendered feet stay on the ground plane.** Camera focus
is derived from that visual foot position.

Animation selection is deterministic: staff strike overrides water casting,
which overrides ember casting, walking, and idle in that order. The staff is
part of the same skin and is rigidly weighted to `staff.R`, so hand, forearm,
upper-arm, and staff motion share one sampled pose.

The walk clip hinges limbs about the character's anatomical left-right world
axis instead of the source bones' rolled local X axes. Its eight contact,
down, passing, and up poses move each ankle through heel strike, planted
loading, toe-off, and dorsiflexed swing clearance. Each authored support pose
measures the four-weight skinned boot and adjusts the pelvis until its sole
meets the ground plane. The poses also add support-side pelvis translation,
pelvis yaw/roll, and a distributed lumbar/thoracic curve: the shoulders stay
ahead of the pelvis, flex farther forward under load, and recover during the up
pose while the head counter-rotates to stabilize the gaze. Both arms swing
opposite their corresponding legs, with elbow flex and a reduced staff-side
arc. Runtime walk phase advances from actual horizontal displacement at 2.75
radians per metre, so collision, terrain, and boundary corrections cannot
leave the feet cycling faster than the player moves. The validation receipt
requires at least 40 cm fore-aft foot travel, 35 degrees of ankle pitch travel,
support-sole error below 6 mm, 20 cm fore-aft hand travel on each side,
2.5–5.5 cm vertical and 2.5–4.5 cm lateral hip travel, upper-body response in
three axes, 3–10 degrees of forward torso lean with at least 2 degrees of
flexion/recovery, and 6–12 degrees of shoulder-versus-pelvis counter-twist.
Idle additionally requires at least 7 cm of foot stagger and
constrains the staff grip and tip outside the skirt at a grounded downward
angle. Water rendering and hit testing both sample the
animated `staff.R` and `staff.tip` transforms from that same pose. Procedural
fruit uses meter-scale constants: apples render and collide at a 10 cm
diameter, independent of tree variation.

Grass patches are world entities rather than render-only decoration. Their
moisture, fuel, ignition, heat transfer, charring, dousing, and burnout all use
the shared reactive solver. The authored recipe varies sphere and capsule
colliders, and the cross-configuration test requires the same ember energy to
ignite both. Each entity consolidates thirteen procedural tufts into one mesh
draw, giving the meadow about two thousand visible tufts while retaining
roughly the former entity/draw count. Rendering reads the resulting state to
shift green grass through heated straw and orange combustion into black char,
and to shrink and lean spent blades; it does not create a second fire state.

The Pocket3D lighting extension is opt-in. The application enables diffuse
bands, wrapped light, rim light, warm/cool ambient balance, and distance fog.
Existing Pocket3D scenes retain their previous defaults.

**A PNG is rendering evidence, not simulation evidence.** The headless command
also writes a receipt with the seed, tick count, state hash, ordered events,
tree state, detached-fruit count, temperatures, fuel, and cooking state.
The Blender receipt separately records asset topology, bone and clip contracts,
rest-pose ground contact, hand and axe travel, and self-contained GLB checks.

## Shelter chemistry

`src/shelter.rs` owns the three trial recipes, material amounts, paired spray
fixtures, sliding controls, camera and HUD. The trials submit the same
`Interaction::Water` packets used by the staff spray in the orchard. The app
authors nozzle paths and visual geometry; it does not select water recipients
or allocate doses. The staff draws three of its actual emitted packet paths;
`World::water_path_distance` clips the center and both side streams at the same
receiver geometry used for delivery. Collision, locomotion and water reception
share world-space capsule endpoints and radius, including nonuniform scale.
The old application-side overlap and dose solver is removed.

`pocket3d-world` owns transformed `TransportSurface` permeability, ordered packet
interception, bounded `Rainfall`, sensible-heat mixing, saturation runoff,
energy-funded evaporation and radiant-heat interception. Thermal contact uses
sphere/capsule surface gaps. Transport boxes do not add rigid-body collision;
sliding panels are application-authored fixtures with authoritative transforms.

**Rain and spray share a finite transported-water budget.** Each packet's mass
is retained, exported as runoff, or leaves its path. Rain samples a fixed spatial
grid, so overlapping receivers cannot multiply input water. Runoff and escaped
water are recorded sinks; this iteration does not simulate puddles or lateral
flow. Surface drying pays latent heat and is bounded by a humidity-dependent
cooling floor. Trial exchange coefficients and material amounts set observable
reaction times in this normalized model.

**Screens intercept an existing radiant-energy share.** A blocked target does
not donate additional heat to other targets. Heat intercepted by a reactive
barrier enters its thermal state; an inert barrier exports that heat. A panel
can block radiation and liquid water with separate coefficients.

The world reports per-target water delivery, heat reception/interception and
ignition conditions. The app accumulates these receipts for the HUD and tests.
The external weather sample and trial controls remain application-owned. The
headless path runs normal frame/input/tick calls and compares complete receipts
on replay. The default orchard and character asset contract have separate
regressions alongside the eight new comparisons.

## Clickable control window

`ui/main.tsx` is a PocketJS application rendered over the scene by
`pocket-ui-wgpu::UiOverlay`. It consumes the shared XP theme and sends action
intents over the in-process overlay channel. `src/controls.rs` owns the view
model, commands, comparison schedules and acceptance state. Automatic and
headless shelter sequences share one action schedule. They submit normal
interactions, inspect world state, then pause for observation.

The native host honors `Game::cursor_mode`: the window requests a visible
pointer and the game requests capture. Mode changes discard held input; the
application also blocks movement/action polling while the window owns input.
Closing resumes simulation and stops a pending automatic comparison. Focus
loss cancels a pointer press instead of producing a click.

`Input` retains ordered pointer edges, so a full click between two frames is
preserved. The overlay maps physical coordinates to the same logical scale as
its renderer. Tests locate named controls through the core's painted bounds,
then inject real cursor/button edges. They cover drags, cancellation, disabled
controls, viewport changes and 1x/2x scaling. A scene camera uses the window's
painted bounds to keep the paired fixtures visible beside it.

`build.rs` invokes the pinned Bun toolchain and PocketJS compiler before Cargo
compiles the application. The JS/PAK pair and build manifest live under Cargo's
`OUT_DIR` and are embedded in the executable. Compiler-reported dependencies
drive rebuilds when application modules, shared framework code, dependencies
or fonts change. Missing output files trigger regeneration; failed generation
fails the Cargo build. No generated bundle is read from the source tree.

`--ui-build-info` emits the manifest embedded in the executable. Acceptance
compares its application/compiler/font input hashes to the current checkout,
so an old executable cannot borrow a newer sidecar manifest. Full receipts,
PNGs and logs are CI artifacts, not versioned test baselines. The repository
owns scenarios and assertions; the workflow stores each run's evidence.

`src/hud.rs` supplies gameplay display data to `ui/hud.tsx`. Both the passive
HUD and the interactive window use the same overlay guest, Inter Regular/Bold
atlases and logical display scale. `ui/text.tsx` wraps paragraphs using the
same native font slot that paints them; notification widths use that provider's
measurement instead of character counts. The presentation model refreshes after
scene composition so projected labels and state readings match the rendered
frame. Only the open control window receives pointer events. The legacy HUD
pass draws the geometric crosshair; application text uses PocketJS components.

## Growth path

The POC keeps the contracts needed for larger worlds while using intentionally
small algorithms:

- Replace pairwise collider checks with a deterministic spatial grid while
  retaining the same contact records.
- Add sleeping islands and per-system budgets before increasing active-body
  counts.
- Store entity recipes separately from state snapshots; version both schemas.
- Stream chunks through deferred spawn and despawn commands at fixed-turn
  boundaries.
- Add shape casts, oriented boxes, joints, and continuous collision behind the
  existing body and contact interfaces.
- Expose commands, queries, and ordered events to a PocketJS guest while
  keeping collision and reactions authoritative in Rust.
- Batch repeated meshes in the renderer and cull against the camera before
  increasing vegetation density.
- Add animation cross-fades and upper-body action layers while retaining the
  current named-clip contract and fixed-step gameplay state.
- Define renderer feature tiers for desktop wgpu, GLES2, PSP GU, and Vita GXM;
  do not assume the desktop shader path is available on device backends.

The next performance milestone should be stated as an active-entity and
contact budget with captured frame time and memory, not as a map-size claim.
