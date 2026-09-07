# Character assets

## Active local Frieren model

`frieren.glb`, `frieren-previews/`, and `frieren-receipt.json` are ordinary
checked-in Git files. Cloning this private repository is sufficient to build
and run the current character; no post-clone asset download is required.
`import_frieren.py` is the reproducible regeneration path, not a build-time
dependency.

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
- `Walk`: an eight-phase contact/down/passing/up stride with heel strike,
  planted loading, toe-off, swing-foot clearance, skinned-sole grounding,
  pelvis weight transfer, load-responsive lumbar/thoracic flexion, shoulder
  counter-rotation, contralateral arm swing with elbow flex, restrained head
  compensation, and a skirt-clear staff carry constraint.
- `Chop`: a forward staff strike used by the tree-cutting interaction.
- `Cast`: the `F` ember-casting pose.
- `Water`: the `Q` water-casting pose.

The left hand remains the `hand.L` pickup socket. Before grounding, the importer
applies the same four-weight skinning limit used by the runtime GLB exporter.
The receipt rejects a walk whose feet drift beyond the authored weight
transfer, whose ankles remain level instead of rolling through heel strike and
toe-off, whose support boot leaves the ground, whose stride lacks fore-aft
travel, whose pelvis does not shift between support legs, or whose upper body
lacks vertical response, torso compression, lateral counter-balance, shoulder
counter-rotation, measurable hand travel on either side, or a forward torso
lean that flexes under load and recovers on the up pose. It also
rejects a rigid attention-style Idle pose, an Idle staff crossing the skirt
silhouette, or a Water pose that does not aim the staff forward. The importer
embeds the source texture as a glTF PBR base-color texture, writes
`frieren-receipt.json`, and renders nine images under
`frieren-previews/` for visual QA.

The BOOTH page does not state an explicit redistribution license. The source
and derived GLB are not covered by this repository's MIT license. Keep them in
this private repository only; do not place them in public repositories,
releases, packages, or other redistribution channels without separate
permission and copyright review.

## Original explorer

`generate_character.py`, `explorer.blend`, `explorer.glb`, `receipt.json`, and
the original previews remain the repository-owned MIT-licensed character
source. They are no longer loaded by the application.
