# Clean-room chibi Frieren

Active files: `generate_chibi.py`, `frieren-chibi.blend`, `frieren-chibi.glb`,
`frieren-chibi-receipt.json`, `chibi-water-sockets.json`, and `chibi-previews/`.
Everything needed to run is checked in; Blender is only a regeneration tool.

```sh
/Applications/Blender.app/Contents/MacOS/Blender \
  --background --factory-startup --python-exit-code 1 --python assets/character/generate_chibi.py
/Applications/Blender.app/Contents/MacOS/Blender \
  --background --factory-startup --python-exit-code 1 --python assets/character/validate_chibi.py
```

Use `-- --output-dir /tmp/chibi --skip-previews` for an independent rebuild.
The generator starts from an empty scene and authors all geometry, solid-color
materials, a 23-joint rig, skin bindings and seven animation clips. It loads
only this repository's original primitive/studio Python helpers. No downloaded
model, texture, rig or animation is read. The proportions are about 2.5 heads.

| Clip | Runtime purpose |
| --- | --- |
| Idle | Grounded breathing and staff carry |
| Walk | Distance-driven stride, articulated knees and arms |
| Chop | Staff strike |
| Cast | Ember casting |
| Water | Staff aimed forward, generated outlet samples |
| Climb | Alternating hands/feet, staff stowed on back |
| Fall | Released/jumping pose |

The application retains `hand.L` for apples, `staff.R` for the staff grip and
`staff.tip` for the water outlet. Each grounded authored frame measures
skinned boot vertices before applying root grounding. The generated receipt
records topology, clip duration/motion, sockets and GLB SHA-256. The GLB contract
test checks skinning, required joints, clip names, staff weights and budgets.
Fresh-import QA checks every frame for finite deformations, grounded soles and
loop seams at the authored 30 fps with NLA disabled. Nine Blender views and separate game-rendered scenes cover visual acceptance.

The original MIT explorer generator and artifacts remain available. See
[ATTRIBUTION.md](../../ATTRIBUTION.md) for the character-design attribution and
provenance of the retired downloaded model in earlier Git history.
