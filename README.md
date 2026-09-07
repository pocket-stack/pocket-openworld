# Pocket Openworld

This repository contains an original Pocket3D proof of concept for a small
systemic world. A deterministic simulation owns bodies, attachments,
structural damage, heat, moisture, fuel, and combustion. The Pocket3D adapter
maps simulation state to procedural low-poly geometry, particles, lighting,
a third-person camera, a clean-room chibi Frieren model, and a debug HUD.

The implementation does not include or derive game assets, source code,
configuration data, shaders, or numeric tuning from *The Legend of Zelda:
Breath of the Wild*. Public reverse-engineering research was used only to
identify general relationships worth testing: physical contacts become ordered
events; physical and reactive materials are separate; attachments can become
independent bodies; and structural or thermal changes produce deferred state
transitions.

## Run

Clone the PocketJS engine dependency with the repository:

```sh
git clone --recurse-submodules git@github.com:pocket-stack/pocket-openworld.git
cd pocket-openworld
cargo run --locked
```

For an existing checkout, run `git submodule update --init --recursive` before
building. **The `vendor/pocketjs` gitlink pins the exact PocketJS revision used
by the application; engine changes remain owned by the PocketJS repository.**

Controls:

- `WASD` moves Frieren. Hold `C` at a trunk or steep slope to grip; `WASD` then climbs up/down or traverses sideways. Release `C` to let go and free the staff hand.
- `Left Shift` jumps from the ground or away from the climbed surface. Stamina recovers on walkable ground; wet or hot surfaces may not support a grip.
- Mouse movement or arrow keys orbit the camera.
- `Space` performs a staff strike at the nearest tree or log.
- `F` plays the staff-casting animation and casts an ember at the aimed
  reactive object, including grass.
- `Q` raises the staff and fires a short forward water burst from its animated
  ornate tip. The stream
  always appears, even over empty ground, and douses every reactive object
  inside its widening corridor.
- `E` picks up or drops the nearest apple.
- `R` resets the world to the initial seed.
- `Escape` releases or captures the mouse; close the window to quit.

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

The checked-in `assets/character/frieren-chibi.glb` is authored from scratch in
Blender: approximately 2.5 heads tall, silver twin tails, pointed ears, jade
eyes, an ivory/gold coat and a garnet staff. Its geometry, 23-joint rig,
materials and animation are generated without loading the previous model,
texture or skeleton. Normal builds need no Blender or asset download.

The `Idle`, `Walk`, `Chop`, `Cast`, and `Water` interaction clips remain;
`Climb` and `Fall` add traversal poses. `hand.L`, `staff.R`, and `staff.tip`
retain their pickup, grip and water-outlet contracts. Climbing stows the staff
on the back. Walk and climb cycle phases follow actual movement distance.

Regenerate the editable `.blend`, GLB, socket samples, receipt and nine previews:

```sh
/Applications/Blender.app/Contents/MacOS/Blender \
  --background --factory-startup --python-exit-code 1 --python assets/character/generate_chibi.py
```

See [character provenance](assets/character/README.md) and [attribution](ATTRIBUTION.md).

## Climbing acceptance

The hill east of the orchard contains walkable ground and steep faces. Trees
and the hill use the same upstream contact motor; no tree positions or terrain
heights are used to move the player in application code. Wetness and heat affect
grip through the existing reactive state. Tree canopies become translucent
when their bounds obstruct the camera's view of the character.

```sh
python3 tools/verify_traversal.py --binary target/debug/pocket-openworld
```

This runs climbing, sideways traversal, release, slopes, action poses, apple
carrying and sustained extinguishing with JSON and PNG evidence under
`evidence/chibi-climbing/`. The receipts track support normals, locomotion
modes, height and stamina. These are desktop/headless receipts, not physical
handheld-device acceptance.

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
checks the seven clip names, required joints, skinned primitives, authored
materials, triangle budget, animation priority, camera target, foot-to-ground
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
