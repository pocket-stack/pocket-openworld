# World Law character models

The World Law POC loads three checked-in runtime GLBs from this directory:

- `eva-unit-01.glb`: a grounded, six-metre EVA Unit-01 visual.
- `colossal-titan-body.glb`: a grounded, ten-metre Colossal Titan with its
  screen-right arm removed.
- `colossal-titan-right-arm.glb`: the matching posed arm, rooted at its
  shoulder so the simulation can detach, drop, and regenerate it.

`import_models.py` is the reproducible adaptation path. It verifies the exact
source hashes, removes viewer helpers, applies scale normalization, poses the
Titan's Mixamo arm rig, bakes the deformation, splits the arm by skin weights,
embeds the original materials, writes `model-receipt.json`, and renders the two
images under `previews/`.

## Sources and attribution

The EVA model is **EVANGELION UNIT ONE** by **BROWNCOAT**, licensed CC BY 4.0:
<https://sketchfab.com/3d-models/evangelion-unit-one-07081cd3a70e494095271c43a591af81>.
The source description additionally gives credit to
<https://byneet.fanbox.cc/>; that credit is preserved here and in the receipt.

The Titan model is **Colossal Titan** by **Sidaivan**, licensed CC BY 4.0:
<https://sketchfab.com/3d-models/colossal-titan-e031a57fd4bf411f8e893361676b4544>.

Both exact GLBs are present in the official Objaverse 1.0 archive. Rebuild with:

```sh
mkdir -p /tmp/pocket-world-law-models
curl -fL \
  -o /tmp/pocket-world-law-models/eva-unit-01-source.glb \
  https://huggingface.co/datasets/allenai/objaverse/resolve/main/glbs/000-017/07081cd3a70e494095271c43a591af81.glb
curl -fL \
  -o /tmp/pocket-world-law-models/colossal-titan-source.glb \
  https://huggingface.co/datasets/allenai/objaverse/resolve/main/glbs/000-028/e031a57fd4bf411f8e893361676b4544.glb

/Applications/Blender.app/Contents/MacOS/Blender \
  --background --factory-startup \
  --python assets/world-law/import_models.py -- \
  --eva-source /tmp/pocket-world-law-models/eva-unit-01-source.glb \
  --titan-source /tmp/pocket-world-law-models/colossal-titan-source.glb
```

The model adaptations remain under CC BY 4.0 rather than this repository's
MIT license. Attribution and the indication of changes above must accompany
redistribution. Evangelion and Attack on Titan character designs, names, and
associated marks remain the property of their respective rights holders; the
source model licenses do not grant rights beyond the contributors' work.
