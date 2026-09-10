# Pocket Openworld

This repository contains an original Pocket3D proof of concept for a small
systemic world. A deterministic simulation owns bodies, attachments,
structural damage, heat, moisture, fuel, and combustion. The Pocket3D adapter
maps simulation state to procedural low-poly geometry, particles, lighting,
a third-person camera, a locally imported rigged Frieren model, and a debug HUD.

The implementation does not include or derive game assets, source code,
configuration data, shaders, or numeric tuning from *The Legend of Zelda:
Breath of the Wild*. Public reverse-engineering research was used only to
identify general relationships worth testing: physical contacts become ordered
events; physical and reactive materials are separate; attachments can become
independent bodies; and structural or thermal changes produce deferred state
transitions.

## Run

Clone the PocketJS engine dependency with the repository:

Source builds require Rust from `rust-toolchain.toml` and Bun from `.bun-version`.
Cargo installs the pinned JavaScript dependencies and generates the UI before
compiling the game; no manual UI build step or checked-in bundle is required.

```sh
git clone --recurse-submodules git@github.com:pocket-stack/pocket-openworld.git
cd pocket-openworld
cargo run --locked
```

For an existing checkout, run `git submodule update --init --recursive` before
building. **The `vendor/pocketjs` gitlink pins the exact PocketJS revision used
by the application; engine changes remain owned by the PocketJS repository.**

Controls:

- **Tab** opens the XP-style PocketJS control window and releases the cursor.
  Click experiment tabs, actions or automatic comparisons. Drag the title bar
  to move the window. Close it with X, Back to game or Tab to resume gameplay.
- **WASD** moves Frieren while the window is closed; the mouse orbits the camera.
- The Orchard tab offers staff swing, ignition, water and pickup actions.

## Clickable chemistry trials

```sh
cargo run --locked -- --shelter
```

This opens the control window over the laboratory. Choose **Rain**, **Screen**
or **Firebreak**, then click the main comparison button. Each sequence submits
normal actions to the simulation, shows progress, verifies the resulting state
and pauses for inspection. The second button runs an alternate comparison;
Firebreak's dry control proves that both rows burn through without water.

Manual buttons expose rain, ignition, spray and movable shielding. They stop the
current automatic sequence and continue simulation. Reset starts the selected
trial again; Pause/Resume controls observation time. Returning to the game also
resumes simulation. The original Frieren asset and five clips remain in use.

[中文点击验收步骤](docs/shelter-acceptance.md) covers the comparisons and controls.
The window shows live moisture, temperature and whether fire reached the far
end. The world remains visible beside it. Input ownership, ordered pointer
edges, DPI mapping and the XP chrome theme come from PocketJS; this application
owns the panel content, recipes and comparison sequences.

All gameplay HUD text uses Inter Regular/Bold through the same PocketJS text
renderer as the XP window. This includes orchard status, target readings, water
progress, experiment observations, projected labels and notifications. The HUD
scales with the display and does not take the pointer from gameplay.

```sh
cargo build --locked
python3 tools/verify_shelter.py --output target/acceptance/controls
```

This reads UI build provenance from the executable, checks its input hashes,
and verifies all 29 scenarios: nine window/input cases,
eight shelter comparisons, ten orchard regressions and two staff-water occlusion cases. Each has an identical
complete receipt replay and a PNG. `cargo build`, `cargo test` and `cargo run`
generate JS, font/style PAK and their manifest under Cargo's `OUT_DIR`.
UI, shared framework and font changes trigger regeneration. The built game
embeds those files and runs without Bun or loose UI assets.

Generated UI and per-run receipts, PNGs and logs are ignored by Git. CI uploads
the full acceptance set as `pocket-openworld-acceptance` artifacts with 30-day
retention; use the workflow run for the PR's current commit. The repository
keeps the scripts, assertions and [manual checklist](docs/shelter-acceptance.md).
[Build verification](docs/build.md) covers fresh generation, incremental changes,
missing outputs, failed compilation and stale-binary detection.

## Deterministic acceptance

The headless path drives the same fixed-step simulation and Pocket3D renderer:

```sh
cargo run --locked -- \
  --headless --scenario orchard-fire --ticks 720 --seed 7 \
  --receipt /tmp/pocket-openworld.json \
  --screenshot /tmp/pocket-openworld.png
```

`--scenario orchard-fire` walks to the tree, chops it, lets attached apples
become rigid bodies, ignites the fallen wood, and records ordered world events.
The receipt proves simulation state; the PNG proves the rendered result.

## Character asset

The complete derived runtime asset is checked into this repository. A fresh
clone receives `assets/character/frieren.glb`, its nine QA previews, the
machine-readable receipt, and the importer; normal builds do not need Blender,
a BOOTH account, or the original `.blend` file.

The active local character is generated from dedastore's free BOOTH
`frieren (.fbx .blend)` download. The importer preserves its 61-joint skin and
adds one staff-tip socket. The runtime selects five named glTF clips:

- `Idle` is a looping at-ease stance with staggered feet and the staff resting
  outside the skirt silhouette.
- `Walk` is an eight-phase game-style stride with planted-foot compression,
  heel strike, toe-off, swing-foot clearance, planted-foot grounding, lateral
  weight transfer, pelvis rotation, counter-rotating shoulders, contralateral
  arm swing with elbow flex, load-responsive lumbar/thoracic flexion, head
  stabilization, and footfall phase tied to actual distance travelled.
- `Chop` is a non-looping staff strike and recovery.
- `Cast` is selected by the `F` ember action.
- `Water` is selected by the `Q` water action.

To reproduce or modify the derived files, first obtain the source model from
BOOTH and regenerate the embedded runtime GLB, nine studio previews, and
machine-readable validation receipt with:

```sh
/Applications/Blender.app/Contents/MacOS/Blender \
  --background --factory-startup \
  --python assets/character/import_frieren.py -- \
  --source "/Users/evan/Downloads/friren_1.1/frieren model.blend"
```

The BOOTH page does not provide an explicit redistribution license. The source
and derived model bytes are not covered by this repository's MIT license; see
`ATTRIBUTION.md` and `assets/character/README.md` before publishing them.

## Character acceptance

These headless scenarios drive the normal `Input`, fixed-step update, animation
selection, GLB skinning, and Pocket3D renderer. They capture the exact poses
used for visual review:

```sh
cargo run --locked -- \
  --headless --scenario idle --ticks 1 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-idle.png

cargo run --locked -- \
  --headless --scenario character-walk --ticks 45 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-walk.png

cargo run --locked -- \
  --headless --scenario character-chop --ticks 119 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-chop.png

cargo run --locked -- \
  --headless --scenario character-cast --ticks 24 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-cast.png

cargo run --locked -- \
  --headless --scenario character-carry --ticks 360 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-carry.png

cargo run --locked -- \
  --headless --scenario character-water --ticks 47 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-water.png

cargo run --locked -- \
  --headless --scenario grass-fire --ticks 48 --size 1440x900 \
  --receipt /tmp/pocket-openworld-grass-fire.json \
  --screenshot /tmp/pocket-openworld-grass-fire.png

cargo run --locked -- \
  --headless --scenario grass-burnout --ticks 210 --size 1440x900 \
  --receipt /tmp/pocket-openworld-grass-burnout.json \
  --screenshot /tmp/pocket-openworld-grass-burnout.png

cargo run --locked -- \
  --headless --scenario campfire-douse --ticks 390 --size 1440x900 \
  --receipt /tmp/pocket-openworld-campfire-douse.json
```

`cargo test --locked --package pocket-openworld` also parses the local GLB and
checks the five clip names, required joints, skinned primitives, embedded
texture, triangle budget, animation priority, camera target, foot-to-ground
transform, hand socket, staff binding, and water corridor. The
`character-cast`, `character-carry`, `character-water`, `grass-fire`, and
`grass-burnout` runs also fail unless their intended live or persistent state
is present at the captured frame. Grass coverage uses the same reactive
simulation for sphere and capsule patch colliders; the test matrix ignites
both configurations without tag-specific solver branches. Each reactive grass
entity renders a thirteen-tuft meadow
patch as one consolidated model draw, so the world contains about two thousand
visible tufts without multiplying draw calls by thirteen. Fresh, burning,
charred, and burned-out states visibly progress from green to orange flame,
then dark, collapsed vegetation.

`campfire-douse` places the character at close range without changing the world
material rules, waits for the two ordinary logs to ignite, and sends one
deterministic Q burst through the curved spray tube. Its receipt records emitted
and delivered water and fails unless the flame plus both logs are extinguished,
the per-tick water budget is conserved, and all three remain out for at least
three seconds after spraying stops.
