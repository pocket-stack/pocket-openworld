//! An original systemic orchard built on `pocket3d-world` + Pocket3D.

mod art;
mod game;

use std::path::PathBuf;

use anyhow::{Context, Result, bail, ensure};
use game::{WorldGame, apply_campfire_douse_script, apply_carry_script, apply_orchard_script};
use pocket3d::app::{AppConfig, Game};
use pocket3d::gpu::{Gpu, OFFSCREEN_FORMAT, OffscreenTarget};
use pocket3d::input::Input;
use pocket3d::renderer::Renderer;
use winit::keyboard::KeyCode;

const SCENARIOS: &[&str] = &[
    "orchard-fire",
    "idle",
    "character-walk",
    "character-chop",
    "character-cast",
    "character-carry",
    "character-water",
    "grass-fire",
    "grass-burnout",
    "campfire-douse",
];
const CHARACTER_CARRY_CAPTURE_TICK: u64 = 360;
const CHARACTER_CAST_CAPTURE_TICK: u64 = 24;
const CHARACTER_WATER_CAPTURE_TICK: u64 = 47;
const GRASS_FIRE_CAPTURE_TICK: u64 = 48;
const GRASS_BURNOUT_CAPTURE_TICK: u64 = 210;

#[derive(Debug)]
struct Args {
    headless: bool,
    scenario: String,
    ticks: u64,
    seed: u64,
    size: (u32, u32),
    screenshot: Option<PathBuf>,
    receipt: Option<PathBuf>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            headless: false,
            scenario: "orchard-fire".into(),
            ticks: 720,
            seed: 7,
            size: (1440, 900),
            screenshot: None,
            receipt: None,
        }
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = parse_args()?;
    if args.headless || args.screenshot.is_some() || args.receipt.is_some() {
        run_headless(args)
    } else {
        pocket3d::app::run(
            AppConfig {
                title: "Pocket3D — Reactive Orchard".into(),
                size: args.size,
                tick_hz: 60.0,
                capture_mouse: true,
                max_fps: Some(60.0),
                ..Default::default()
            },
            WorldGame::new(args.seed),
        )
    }
}

fn run_headless(args: Args) -> Result<()> {
    ensure!(args.ticks > 0, "--ticks must be positive");
    if !SCENARIOS.contains(&args.scenario.as_str()) {
        bail!(
            "unknown scenario {:?}; expected one of {}",
            args.scenario,
            SCENARIOS.join(", ")
        );
    }
    if args.scenario == "character-carry" {
        ensure!(
            args.ticks == CHARACTER_CARRY_CAPTURE_TICK,
            "character-carry requires --ticks {CHARACTER_CARRY_CAPTURE_TICK} so the captured frame proves the held apple"
        );
    }
    if args.scenario == "character-cast" {
        ensure!(
            args.ticks == CHARACTER_CAST_CAPTURE_TICK,
            "character-cast requires --ticks {CHARACTER_CAST_CAPTURE_TICK} so the captured frame proves the active staff cast"
        );
    }
    if args.scenario == "character-water" {
        ensure!(
            args.ticks == CHARACTER_WATER_CAPTURE_TICK,
            "character-water requires --ticks {CHARACTER_WATER_CAPTURE_TICK} so the captured frame proves the active water burst"
        );
    }
    if args.scenario == "grass-fire" {
        ensure!(
            args.ticks == GRASS_FIRE_CAPTURE_TICK,
            "grass-fire requires --ticks {GRASS_FIRE_CAPTURE_TICK} so the captured frame proves active vegetation combustion"
        );
    }
    if args.scenario == "grass-burnout" {
        ensure!(
            args.ticks == GRASS_BURNOUT_CAPTURE_TICK,
            "grass-burnout requires --ticks {GRASS_BURNOUT_CAPTURE_TICK} so the captured frame proves persistent collapsed char"
        );
    }
    let gpu = Gpu::new_headless()?;
    let mut renderer = Renderer::new(&gpu, OFFSCREEN_FORMAT)?;
    let mut game = WorldGame::new(args.seed);
    game.init(&gpu, &mut renderer)?;
    if args.scenario == "campfire-douse" {
        game.prepare_campfire_douse_scenario();
    }
    if matches!(args.scenario.as_str(), "grass-fire" | "grass-burnout") {
        game.prepare_grass_fire_scenario();
    }
    let mut input = Input::default();
    for turn in 0..args.ticks {
        apply_scenario_script(&mut input, &args.scenario, turn);
        game.frame(1.0 / 60.0, &input);
        game.tick(1.0 / 60.0, &input);
        input.end_frame();
    }

    if let Some(path) = args.screenshot.as_deref() {
        let target = OffscreenTarget::new(&gpu, args.size.0, args.size.1);
        let (scene, camera, hud) = game.compose(0.0, args.ticks as f32 / 60.0, args.size);
        renderer.render(&gpu, &target.view, args.size, scene, camera, hud);
        target
            .save_png(&gpu, path)
            .with_context(|| format!("writing screenshot {}", path.display()))?;
        println!("pocket-openworld: wrote screenshot {}", path.display());
    }

    let receipt = game.runtime_receipt(args.scenario.clone());
    let receipt_json = serde_json::to_string_pretty(&receipt)?;
    if let Some(path) = args.receipt.as_deref() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(path, receipt_json.as_bytes())
            .with_context(|| format!("writing receipt {}", path.display()))?;
        println!("pocket-openworld: wrote receipt {}", path.display());
    } else {
        println!("{receipt_json}");
    }
    if args.scenario == "orchard-fire" {
        ensure!(
            receipt.acceptance.playable_chain_complete,
            "orchard-fire acceptance failed: {:#?}",
            receipt.acceptance
        );
    }
    if args.scenario == "character-carry" {
        ensure!(
            game.is_holding_apple(),
            "character-carry acceptance failed: no apple attached to the explorer"
        );
    }
    if args.scenario == "character-cast" {
        ensure!(
            game.ember_cast_active(),
            "character-cast acceptance failed: staff casting animation was not active at capture"
        );
    }
    if args.scenario == "character-water" {
        ensure!(
            game.water_burst_active(),
            "character-water acceptance failed: water burst was not active at capture"
        );
    }
    if args.scenario == "grass-fire" {
        ensure!(
            game.grass_ignited(),
            "grass-fire acceptance failed: no grass entity reached ignition"
        );
    }
    if args.scenario == "grass-burnout" {
        ensure!(
            game.grass_burned_out(),
            "grass-burnout acceptance failed: no grass entity retained burned-out char"
        );
    }
    if args.scenario == "campfire-douse" {
        ensure!(
            receipt.water.campfire_douse.passed,
            "campfire-douse acceptance failed: {:#?}",
            receipt.water.campfire_douse
        );
    }
    println!(
        "pocket-openworld: {} turns, state {}, systemic acceptance {}",
        receipt.ticks, receipt.state_hash, receipt.acceptance.playable_chain_complete
    );
    Ok(())
}

fn parse_args() -> Result<Args> {
    let mut args = Args::default();
    let mut values = std::env::args().skip(1);
    while let Some(argument) = values.next() {
        match argument.as_str() {
            "--headless" => args.headless = true,
            "--scenario" => {
                args.scenario = values.next().context("--scenario requires a value")?;
            }
            "--ticks" => {
                args.ticks = values
                    .next()
                    .context("--ticks requires a value")?
                    .parse()
                    .context("--ticks must be an integer")?;
            }
            "--seed" => {
                args.seed = values
                    .next()
                    .context("--seed requires a value")?
                    .parse()
                    .context("--seed must be an integer")?;
            }
            "--size" => {
                args.size = parse_size(&values.next().context("--size requires WIDTHxHEIGHT")?)?;
            }
            "--screenshot" => {
                args.screenshot = Some(PathBuf::from(
                    values.next().context("--screenshot requires a path")?,
                ));
            }
            "--receipt" => {
                args.receipt = Some(PathBuf::from(
                    values.next().context("--receipt requires a path")?,
                ));
            }
            "-h" | "--help" => {
                println!(
                    "pocket-openworld\n\n  --headless\n  --scenario orchard-fire|idle|character-walk|character-chop|character-cast|character-carry|character-water|grass-fire|grass-burnout|campfire-douse\n  --ticks N\n  --seed N\n  --size WIDTHxHEIGHT\n  --screenshot PATH\n  --receipt PATH"
                );
                std::process::exit(0);
            }
            _ => bail!("unknown argument {argument:?}; use --help"),
        }
    }
    ensure!(
        args.size.0 >= 320 && args.size.1 >= 200,
        "--size is too small"
    );
    ensure!(
        args.size.0 <= 4096 && args.size.1 <= 4096,
        "--size is too large"
    );
    Ok(args)
}

fn apply_scenario_script(input: &mut Input, scenario: &str, turn: u64) {
    match scenario {
        "orchard-fire" => apply_orchard_script(input, turn),
        "character-walk" => input.inject_key(KeyCode::KeyW, true),
        "character-chop" => {
            input.inject_key(KeyCode::KeyW, turn < 101);
            input.inject_key(KeyCode::Space, turn == 108);
            if turn == 109 {
                input.inject_key(KeyCode::Space, false);
            }
        }
        "character-cast" => {
            input.inject_key(KeyCode::ArrowLeft, turn < 10);
            input.inject_key(KeyCode::KeyF, turn == 10);
            if turn == 11 {
                input.inject_key(KeyCode::KeyF, false);
            }
        }
        "character-carry" => {
            apply_carry_script(input, turn);
            input.inject_key(KeyCode::KeyS, (302..332).contains(&turn));
            input.inject_key(KeyCode::ArrowLeft, (334..356).contains(&turn));
            // The real-scale fruit settles closer to the trunk than the old
            // oversized proxy, so take a short approach before pickup.
            input.inject_key(
                KeyCode::KeyD,
                (284..292).contains(&turn) || (357..359).contains(&turn),
            );
        }
        "character-water" => {
            input.inject_key(KeyCode::ArrowLeft, turn < 35);
            input.inject_key(KeyCode::KeyQ, turn == 35);
            if turn == 36 {
                input.inject_key(KeyCode::KeyQ, false);
            }
        }
        "grass-fire" | "grass-burnout" => {
            input.inject_key(KeyCode::KeyF, turn == 1);
            if turn == 2 {
                input.inject_key(KeyCode::KeyF, false);
            }
        }
        "campfire-douse" => apply_campfire_douse_script(input, turn),
        "idle" => {}
        _ => unreachable!("scenario was validated before playback"),
    }
}

fn parse_size(value: &str) -> Result<(u32, u32)> {
    let (width, height) = value
        .split_once(['x', 'X'])
        .context("--size must be WIDTHxHEIGHT")?;
    Ok((
        width.parse().context("invalid width")?,
        height.parse().context("invalid height")?,
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use gltf::{Semantic, buffer, mesh};

    use super::*;

    #[test]
    fn character_preview_scripts_exercise_live_input_paths() {
        let mut walk = Input::default();
        apply_scenario_script(&mut walk, "character-walk", 24);
        assert!(walk.key_down(KeyCode::KeyW));

        let mut chop = Input::default();
        apply_scenario_script(&mut chop, "character-chop", 108);
        assert!(chop.key_down(KeyCode::Space));

        let mut cast = Input::default();
        apply_scenario_script(&mut cast, "character-cast", 10);
        assert!(cast.key_down(KeyCode::KeyF));

        let mut carry = Input::default();
        apply_scenario_script(&mut carry, "character-carry", 300);
        assert!(carry.key_down(KeyCode::KeyE));

        let mut water = Input::default();
        apply_scenario_script(&mut water, "character-water", 35);
        assert!(water.key_down(KeyCode::KeyQ));

        let mut grass = Input::default();
        apply_scenario_script(&mut grass, "grass-fire", 1);
        assert!(grass.key_down(KeyCode::KeyF));

        let mut spent_grass = Input::default();
        apply_scenario_script(&mut spent_grass, "grass-burnout", 1);
        assert!(spent_grass.key_down(KeyCode::KeyF));

        let mut campfire = Input::default();
        apply_scenario_script(&mut campfire, "campfire-douse", 120);
        assert!(campfire.key_down(KeyCode::KeyQ));
    }

    #[test]
    fn frieren_glb_contains_the_runtime_rig_contract() {
        let bytes = include_bytes!("../assets/character/frieren.glb");
        let gltf = gltf::Gltf::from_slice(bytes).expect("frieren.glb must parse");
        assert!(
            gltf.blob.is_some(),
            "the runtime GLB must be self-contained"
        );
        assert!(
            gltf.buffers()
                .all(|buffer| matches!(buffer.source(), buffer::Source::Bin))
        );

        let animation_names: BTreeSet<_> = gltf
            .animations()
            .map(|animation| animation.name().unwrap_or("<unnamed>"))
            .collect();
        assert_eq!(
            animation_names,
            BTreeSet::from(["Cast", "Chop", "Idle", "Walk", "Water"])
        );
        let clip_targets = |name: &str| -> BTreeSet<String> {
            gltf.animations()
                .find(|animation| animation.name() == Some(name))
                .expect("required clip was checked above")
                .channels()
                .filter_map(|channel| channel.target().node().name().map(str::to_owned))
                .collect()
        };
        let walk_targets = clip_targets("Walk");
        assert!(walk_targets.contains("leg.L") && walk_targets.contains("leg.R"));
        let chop_targets = clip_targets("Chop");
        assert!(
            chop_targets.contains("Arm.R") && chop_targets.contains("foreArm.R"),
            "Chop must animate Frieren's staff-side arm chain"
        );
        let cast_targets = clip_targets("Cast");
        assert!(cast_targets.contains("staff.R") && cast_targets.contains("hand.L"));
        let water_targets = clip_targets("Water");
        assert!(
            water_targets.contains("staff.R")
                && water_targets.contains("staff.tip")
                && water_targets.contains("hand.L")
        );

        let joint_names: BTreeSet<_> = gltf
            .skins()
            .flat_map(|skin| skin.joints())
            .filter_map(|node| node.name())
            .collect();
        for required in [
            "Hips",
            "Spine",
            "Spine1",
            "Spine2",
            "Neck",
            "Head",
            "Arm.L",
            "foreArm.L",
            "hand.L",
            "Arm.R",
            "foreArm.R",
            "hand.R",
            "leg.L",
            "knee.L",
            "foot.L",
            "leg.R",
            "knee.R",
            "foot.R",
            "staff.R",
            "staff.tip",
        ] {
            assert!(
                joint_names.contains(required),
                "missing required joint {required}"
            );
        }

        assert!(
            gltf.nodes()
                .filter(|node| node.mesh().is_some())
                .all(|node| node.skin().is_some()),
            "every Frieren runtime mesh, including the staff, must be driven by a skin"
        );

        let mut triangles = 0_usize;
        let mut primitive_count = 0_usize;
        let mut skinned_primitives = 0_usize;
        for primitive in gltf.meshes().flat_map(|mesh| mesh.primitives()) {
            primitive_count += 1;
            assert_eq!(primitive.mode(), mesh::Mode::Triangles);
            triangles += primitive
                .indices()
                .or_else(|| primitive.get(&Semantic::Positions))
                .map(|accessor| accessor.count() / 3)
                .unwrap_or_default();
            if primitive.get(&Semantic::Joints(0)).is_some()
                && primitive.get(&Semantic::Weights(0)).is_some()
            {
                skinned_primitives += 1;
            }
        }
        assert!(
            (2_000..=8_000).contains(&triangles),
            "Frieren mesh budget changed: {triangles} triangles"
        );
        assert_eq!(
            primitive_count, skinned_primitives,
            "every Frieren primitive must carry skin weights"
        );
        assert!(
            primitive_count <= 16,
            "Frieren draw-call budget changed: {primitive_count} primitives"
        );
        assert_eq!(
            gltf.materials().count(),
            1,
            "Frieren material contract changed"
        );
        assert_eq!(gltf.images().count(), 1, "Frieren texture was not embedded");

        let (document, buffers, _) = gltf::import_slice(bytes).expect("GLB payload must import");
        assert!((1..=4).contains(&document.skins().count()));
        let staff_node = document
            .nodes()
            .find(|node| node.name() == Some("staff"))
            .expect("Frieren GLB must retain the named staff node");
        let staff_skin = staff_node
            .skin()
            .expect("staff node must reference its skin");
        let staff_mesh = staff_node
            .mesh()
            .expect("staff node must reference its mesh");
        let staff_joint = staff_skin
            .joints()
            .position(|joint| joint.name() == Some("staff.R"))
            .expect("staff.R must be in the staff skin") as u16;
        let staff_weighted_vertices = staff_mesh
            .primitives()
            .map(|primitive| {
                let reader = primitive
                    .reader(|buffer| buffers.get(buffer.index()).map(|data| data.0.as_slice()));
                let joints = reader
                    .read_joints(0)
                    .expect("skinned primitive must have JOINTS_0")
                    .into_u16();
                let weights = reader
                    .read_weights(0)
                    .expect("skinned primitive must have WEIGHTS_0")
                    .into_f32();
                joints
                    .zip(weights)
                    .filter(|(joints, weights)| {
                        (0..4).any(|index| joints[index] == staff_joint && weights[index] > 0.999)
                    })
                    .count()
            })
            .sum::<usize>();
        assert!(
            staff_weighted_vertices >= 100,
            "staff geometry is no longer rigidly bound to staff.R: {staff_weighted_vertices} vertices"
        );
    }
}
