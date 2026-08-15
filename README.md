# Pocket Openworld

This repository contains an original Pocket3D proof of concept for a small
systemic world. A deterministic simulation owns bodies, attachments,
structural damage, heat, moisture, fuel, and combustion. The Pocket3D adapter
maps simulation state to procedural low-poly geometry, particles, lighting,
a third-person camera, a rigged original explorer, and a debug HUD.

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

- `WASD` moves the explorer.
- Mouse movement or arrow keys orbit the camera.
- `Space` swings the axe at the nearest tree or log.
- `F` casts an ember at the aimed reactive object.
- `Q` fires a short forward water burst from the explorer's waist. The stream
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

## Explorer asset

The explorer is an original Blender-generated character with a 19-joint skin,
modeled face and hair, layered clothing, articulated hands, and an axe bound to
the right-hand chain. The runtime selects three named glTF clips:

- `Idle` is a looping breathing and look pose.
- `Walk` is a looping stride with opposing arm motion.
- `Chop` is a non-looping wind-up, strike, and recovery.

The checked-in generator produces the editable source, embedded runtime GLB,
five studio previews, and a machine-readable validation receipt:

```sh
/Applications/Blender.app/Contents/MacOS/Blender \
  --background --factory-startup \
  --python assets/character/generate_character.py
```

The same script works with a `blender` executable on other platforms. See
`assets/character/README.md` for the output contract and validation details.

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
  --headless --scenario character-chop --ticks 117 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-chop.png

cargo run --locked -- \
  --headless --scenario character-carry --ticks 360 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-carry.png

cargo run --locked -- \
  --headless --scenario character-water --ticks 47 --size 1440x900 \
  --screenshot /tmp/pocket-openworld-character-water.png

cargo run --locked -- \
  --headless --scenario campfire-douse --ticks 390 --size 1440x900 \
  --receipt /tmp/pocket-openworld-campfire-douse.json
```

`cargo test --locked --package pocket-openworld`
also parses the checked-in GLB and checks the clip names, required joints,
skinned primitives, material separation, triangle budget, animation priority,
camera target, foot-to-ground transform, hand socket, and water corridor. The
`character-carry` and `character-water` runs also fail if their interaction is
not active at the captured frame.

`campfire-douse` places the explorer at close range without changing the world
material rules, waits for the two ordinary logs to ignite, and sends one
deterministic Q burst through the curved spray tube. Its receipt records emitted
and delivered water and fails unless the flame plus both logs are extinguished,
the per-tick water budget is conserved, and all three remain out for at least
three seconds after spraying stops.
