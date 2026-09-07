# Chibi character and surface traversal acceptance

The engine dependency is PocketJS `fbd3a5d4` ([PR #380](https://github.com/pocket-stack/pocketjs/pull/380)).

Run from the repository root:

```sh
cargo build --locked
python3 tools/verify_traversal.py
```

The verifier drives the normal input/frame/fixed-tick/render path, checks the
scenario contract and repeats every run to compare the entire JSON receipt.
`summary.json` records the engine, character and executable hashes. PNGs show
rendering; each neighboring JSON file independently records simulation state.
All captures use seed 7, 60 fixed turns/second, and 1200×750 pixels on the
desktop wgpu renderer. Physical handheld-device behavior is outside this run.

| Evidence | Contract |
| --- | --- |
| [Tree ascent](tree-climb.png) · [state](tree-climb.json) | Gripping raises the actor and consumes stamina |
| [Traverse](tree-traverse.png) · [state](tree-traverse.json) | Sideways input follows a curved trunk and changes the support normal |
| [Release](climb-release.png) · [state](climb-release.json) | Releasing C resumes gravity |
| [Jump](climb-jump.png) · [state](climb-jump.json) | Shift jumps away while C remains held |
| [Slope climb](slope-climb.png) · [state](slope-climb.json) | The same motor climbs a heightfield face |
| [Steep slope](slope-walk.png) · [state](slope-walk.json) | Walking without grip slides on steep terrain |
| [Crest](slope-crest.png) · [state](slope-crest.json) | Reaching walkable ground restores walking and stamina recovery |
| [Carry](character-carry.png) · [state](character-carry.json) | A real-scale apple remains visible at hand.L |
| [Water](character-water.png) · [state](character-water.json) | The stream starts at the animated staff tip |
| [Douse](campfire-douse.png) · [state](campfire-douse.json) | Flame and both logs remain extinguished after spraying |

Additional captures retain idle, walking, staff strike, ember casting, the
orchard systemic chain, grass ignition and persistent burnout.

[Blender front view](../../assets/character/chibi-previews/idle-front.png),
[fresh-import deformation QA](../../assets/character/frieren-chibi-import-qa.json),
and [asset receipt](../../assets/character/frieren-chibi-receipt.json) accompany
the runtime captures. Independent Blender regeneration produced a byte-identical
GLB. The exported rig has 23 joints, 7 clips and no external texture inputs.

Local validation uses Rust 1.97.0: 31 application tests, 54 upstream CPU tests,
an explicit GPU opacity pixel test, format checking and Clippy with warnings
denied. The application CI reruns the tests and all 17 scenarios with receipt
replay and uploads its own evidence.
