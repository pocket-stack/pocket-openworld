#!/usr/bin/env python3
"""Fresh-import deformation QA; run with Blender --background --factory-startup."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import sys
import bpy

HERE = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("--asset", type=Path, default=HERE / "frieren-chibi.glb")
parser.add_argument(
    "--output", type=Path, default=HERE / "frieren-chibi-import-qa.json"
)
args = parser.parse_args(
    sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
)
bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete(use_global=False)
bpy.context.scene.render.fps = 30
bpy.ops.import_scene.gltf(filepath=str(args.asset.resolve()))
rigs = [o for o in bpy.context.scene.objects if o.type == "ARMATURE"]
assert len(rigs) == 1
rig = rigs[0]
rig.animation_data_create()
rig.animation_data.use_nla = False
meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
# The Blender importer creates an unskinned bone-widget mesh as UI helper.
meshes = [o for o in meshes if o.vertex_groups]
assert len(meshes) == 2
body = next(o for o in meshes if o.name != "staff")
feet = {g.index for g in body.vertex_groups if g.name in ("foot.L", "foot.R")}
indices = [
    v.index
    for v in body.data.vertices
    if any(g.group in feet and g.weight > 0.5 for g in v.groups)
]
assert len(indices) > 50
result = {}
for action in list(bpy.data.actions):
    name = action.name.split("|")[-1]
    rig.animation_data.action = action
    start, end = map(round, action.frame_range)
    sole_errors = []
    first = None
    last = None
    for frame in range(start, end + 1):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        evaluated = body.evaluated_get(bpy.context.evaluated_depsgraph_get())
        mesh = evaluated.to_mesh()
        vertices = [evaluated.matrix_world @ v.co for v in mesh.vertices]
        assert all(all(math.isfinite(c) for c in v) for v in vertices)
        sole = min(vertices[i].z for i in indices)
        sole_errors.append(abs(sole))
        positions = [tuple(rig.matrix_world @ b.head) for b in rig.pose.bones]
        if first is None:
            first = positions
        last = positions
        evaluated.to_mesh_clear()
    seam = max(math.dist(a, b) for a, b in zip(first, last))
    if name in ("Idle", "Walk", "Chop", "Cast", "Water"):
        assert max(sole_errors) < 0.006, (name, max(sole_errors))
    if name in ("Idle", "Walk", "Climb", "Fall"):
        assert seam < 0.002, (name, seam)
    result[name] = {
        "frames_checked": end - start + 1,
        "max_support_sole_error_m": max(sole_errors),
        "loop_joint_seam_m": seam,
    }
assert set(result) == {"Idle", "Walk", "Chop", "Cast", "Water", "Climb", "Fall"}, (
    result.keys()
)
receipt = {
    "schema": "pocket-openworld.chibi-import-qa.v1",
    "blender": bpy.app.version_string,
    "glb_sha256": hashlib.sha256(args.asset.read_bytes()).hexdigest(),
    "fresh_import": True,
    "mesh_count": len(meshes),
    "bone_count": len(rig.data.bones),
    "clips": result,
}
args.output.write_text(json.dumps(receipt, indent=2) + "\n")
print(json.dumps(receipt, indent=2))
