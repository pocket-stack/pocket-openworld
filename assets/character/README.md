# Character assets

## Active local Frieren model

The local runtime uses `frieren.glb`, derived from dedastore's free
`frieren (.fbx .blend)` BOOTH download. Obtain the source separately from
<https://booth.pm/ja/items/5469071>, then run:

```sh
/Applications/Blender.app/Contents/MacOS/Blender \
  --background --factory-startup \
  --python assets/character/import_frieren.py -- \
  --source "/Users/evan/Downloads/friren_1.1/frieren model.blend"
```

The importer preserves the supplied 61-bone skin, body, face, hair, texture,
and right-hand staff, then adds the `staff.tip` outlet socket. It authors five
Pocket3D clips:

- `Idle`: relaxed breathing in an at-ease, staggered-foot stance with the
  staff grounded outside the skirt.
- `Walk`: a looping stride with opposed limb motion and a skirt-clear staff
  carry constraint.
- `Chop`: a forward staff strike used by the tree-cutting interaction.
- `Cast`: the `F` ember-casting pose.
- `Water`: the `Q` water-casting pose.

The left hand remains the `hand.L` pickup socket. The receipt rejects a walk
whose feet sweep sideways, whose stride lacks fore-aft travel, or whose hips do
not visibly rise and fall. It also rejects a rigid attention-style Idle pose,
an Idle staff crossing the skirt silhouette, or a Water pose that does not aim
the staff forward. The importer embeds the source texture as a glTF PBR base-color
texture, writes
`frieren-receipt.json`, and renders six images under `frieren-previews/` for
visual QA.

The BOOTH page does not state an explicit redistribution license. The source
and derived GLB are not covered by this repository's MIT license. Keep them in
this private repository only; do not place them in public repositories,
releases, packages, or other redistribution channels without separate
permission and copyright review.

## Original explorer

`generate_character.py`, `explorer.blend`, `explorer.glb`, `receipt.json`, and
the original previews remain the repository-owned MIT-licensed character
source. They are no longer loaded by the application.
