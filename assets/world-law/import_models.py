"""Build the World Law POC character GLBs from attributed source models.

The inputs are the exact Sketchfab models archived by Objaverse 1.0. This
script verifies their hashes, normalizes EVA Unit-01 to a six-metre visual
height, poses the Colossal Titan's rig, and splits its screen-right arm into a
separate static mesh for the severing/regeneration presentation.

Example:
  blender --background --factory-startup --python import_models.py -- \
    --eva-source /tmp/eva-unit-01-source.glb \
    --titan-source /tmp/colossal-titan-source.glb
"""

import argparse
import bmesh
import hashlib
import json
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector


HERE = Path(__file__).resolve().parent
PREVIEW_DIR = HERE / "previews"
RECEIPT_PATH = HERE / "model-receipt.json"
EVA_PATH = HERE / "eva-unit-01.glb"
TITAN_BODY_PATH = HERE / "colossal-titan-body.glb"
TITAN_ARM_PATH = HERE / "colossal-titan-right-arm.glb"

EVA_SOURCE_SHA256 = "fc6a86ec438b5734d892da66ccb5a5c0419b0c9c2e18ac6f82754b7a51268629"
TITAN_SOURCE_SHA256 = "db62cec0a2f9c666e589a06dd3e6fde76bf1cf9aac76e9f5fadf2a5b89baf238"
EVA_HEIGHT_M = 6.0
TITAN_HEIGHT_M = 10.0


def arguments():
    parser = argparse.ArgumentParser(description="Build World Law character assets")
    parser.add_argument("--eva-source", type=Path, required=True)
    parser.add_argument("--titan-source", type=Path, required=True)
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    return parser.parse_args(args)


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require_source(path, expected_sha256, label):
    if not path.is_file():
        raise RuntimeError(f"{label} source does not exist: {path}")
    actual = sha256(path)
    if actual != expected_sha256:
        raise RuntimeError(
            f"{label} source SHA-256 mismatch: expected {expected_sha256}, got {actual}"
        )


def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def import_glb(path):
    bpy.ops.import_scene.gltf(filepath=str(path))
    return list(bpy.context.scene.objects)


def mesh_objects():
    return sorted(
        (obj for obj in bpy.context.scene.objects if obj.type == "MESH"),
        key=lambda obj: obj.name,
    )


def remove_helpers(names):
    for obj in list(bpy.context.scene.objects):
        if obj.type == "MESH" and obj.name in names:
            bpy.data.objects.remove(obj, do_unlink=True)


def world_bounds(objects):
    points = [obj.matrix_world @ vertex.co for obj in objects for vertex in obj.data.vertices]
    if not points:
        raise RuntimeError("asset contains no renderable bounds")
    low = Vector((min(p.x for p in points), min(p.y for p in points), min(p.z for p in points)))
    high = Vector((max(p.x for p in points), max(p.y for p in points), max(p.z for p in points)))
    return low, high


def bake_object(obj, name):
    mesh = obj.data.copy()
    mesh.transform(obj.matrix_world)
    baked = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(baked)
    return baked


def normalize_grounded(objects, height_m):
    low, high = world_bounds(objects)
    height = high.z - low.z
    if height <= 0.0:
        raise RuntimeError("asset has zero height")
    origin = Vector(((low.x + high.x) * 0.5, (low.y + high.y) * 0.5, low.z))
    scale = height_m / height
    transform = Matrix.Scale(scale, 4) @ Matrix.Translation(-origin)
    for obj in objects:
        obj.data.transform(transform @ obj.matrix_world)
        obj.matrix_world = Matrix.Identity(4)
    return origin, scale


def select_only(objects):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]


def export_glb(objects, path):
    select_only(objects)
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_cameras=False,
        export_lights=False,
        export_apply=True,
        export_yup=True,
    )


def polygon_count(objects):
    return sum(len(obj.data.polygons) for obj in objects)


def vertex_count(objects):
    return sum(len(obj.data.vertices) for obj in objects)


def output_receipt(path, objects):
    low, high = world_bounds(objects)
    # Blender is Z-up; glTF/runtime is Y-up and maps Blender -Y to +Z.
    runtime_low = [low.x, low.z, -high.y]
    runtime_high = [high.x, high.z, -low.y]
    return {
        "file": path.name,
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
        "triangles": polygon_count(objects),
        "vertices": vertex_count(objects),
        "runtime_aabb_m": {
            "min": [round(v, 5) for v in runtime_low],
            "max": [round(v, 5) for v in runtime_high],
        },
    }


def prepare_eva(source):
    reset_scene()
    import_glb(source)
    remove_helpers({"Cube"})
    source_meshes = mesh_objects()
    if not (20 <= len(source_meshes) <= 30):
        raise RuntimeError(f"unexpected EVA mesh count: {len(source_meshes)}")
    baked = [bake_object(obj, f"eva-unit-01-{index:02}") for index, obj in enumerate(source_meshes)]
    for obj in source_meshes:
        bpy.data.objects.remove(obj, do_unlink=True)
    normalize_grounded(baked, EVA_HEIGHT_M)
    export_glb(baked, EVA_PATH)
    receipt = output_receipt(EVA_PATH, baked)
    render_preview(baked, PREVIEW_DIR / "eva-unit-01.png", portrait=True)
    return receipt


def aim_pose_bone(armature, name, desired_direction):
    pose_bone = armature.pose.bones[name]
    bpy.context.view_layer.update()
    head = pose_bone.head.copy()
    current = (pose_bone.tail - head).normalized()
    correction = current.rotation_difference(Vector(desired_direction).normalized())
    pose_bone.matrix = (
        Matrix.Translation(head)
        @ correction.to_matrix().to_4x4()
        @ Matrix.Translation(-head)
        @ pose_bone.matrix
    )
    bpy.context.view_layer.update()


def titan_arm_group_indices(obj):
    # The source model's screen-right arm uses Mixamo's anatomical Left chain.
    prefixes = (
        "mixamorig_LeftArm_",
        "mixamorig_LeftForeArm_",
        "mixamorig_LeftHand_",
        "mixamorig_LeftHandThumb",
        "mixamorig_LeftHandIndex",
        "mixamorig_LeftHandMiddle",
        "mixamorig_LeftHandRing",
        "mixamorig_LeftHandPinky",
    )
    return {group.index for group in obj.vertex_groups if group.name.startswith(prefixes)}


def titan_arm_weights(obj):
    indices = titan_arm_group_indices(obj)
    return [
        sum(membership.weight for membership in vertex.groups if membership.group in indices)
        for vertex in obj.data.vertices
    ]


def delete_weighted_vertices(mesh, weights, keep_arm):
    if len(mesh.vertices) != len(weights):
        raise RuntimeError("posed Titan topology no longer matches source vertex weights")
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bm.verts.ensure_lookup_table()
    if keep_arm:
        doomed = [vertex for vertex in bm.verts if weights[vertex.index] < 0.20]
    else:
        doomed = [vertex for vertex in bm.verts if weights[vertex.index] >= 0.20]
    bmesh.ops.delete(bm, geom=doomed, context="VERTS")
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()


def evaluated_mesh(obj, depsgraph, name):
    evaluated = obj.evaluated_get(depsgraph)
    mesh = bpy.data.meshes.new_from_object(evaluated, depsgraph=depsgraph)
    mesh.name = name
    baked = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(baked)
    baked.matrix_world = obj.matrix_world.copy()
    return baked


def prepare_titan(source):
    reset_scene()
    import_glb(source)
    remove_helpers({"Cube", "Icosphere"})
    armatures = [obj for obj in bpy.context.scene.objects if obj.type == "ARMATURE"]
    if len(armatures) != 1:
        raise RuntimeError(f"expected one Titan armature, found {len(armatures)}")
    armature = armatures[0]
    skinned = [
        obj
        for obj in mesh_objects()
        if any(modifier.type == "ARMATURE" for modifier in obj.modifiers)
    ]
    if len(skinned) != 3:
        raise RuntimeError(f"expected three Titan skinned meshes, found {len(skinned)}")

    # Bring both arms down from the source T-pose. The right-screen arm is
    # split later, while the opposite arm remains baked into the body mesh.
    aim_pose_bone(armature, "mixamorig_LeftArm_09", (0.24, 0.02, -0.97))
    aim_pose_bone(armature, "mixamorig_LeftForeArm_010", (0.14, -0.08, -0.99))
    aim_pose_bone(armature, "mixamorig_RightArm_033", (-0.24, 0.02, -0.97))
    aim_pose_bone(armature, "mixamorig_RightForeArm_034", (-0.14, -0.08, -0.99))

    depsgraph = bpy.context.evaluated_depsgraph_get()
    depsgraph.update()
    shoulder_world = armature.matrix_world @ armature.pose.bones["mixamorig_LeftArm_09"].head
    body = []
    right_arm = []
    for index, source_obj in enumerate(skinned):
        weights = titan_arm_weights(source_obj)
        body_obj = evaluated_mesh(source_obj, depsgraph, f"colossal-titan-body-{index:02}")
        arm_obj = evaluated_mesh(source_obj, depsgraph, f"colossal-titan-right-arm-{index:02}")
        delete_weighted_vertices(body_obj.data, weights, keep_arm=False)
        delete_weighted_vertices(arm_obj.data, weights, keep_arm=True)
        if body_obj.data.polygons:
            body.append(body_obj)
        else:
            bpy.data.objects.remove(body_obj, do_unlink=True)
        if arm_obj.data.polygons:
            right_arm.append(arm_obj)
        else:
            bpy.data.objects.remove(arm_obj, do_unlink=True)

    for source_obj in skinned:
        bpy.data.objects.remove(source_obj, do_unlink=True)
    bpy.data.objects.remove(armature, do_unlink=True)

    posed_objects = body + right_arm
    low, high = world_bounds(posed_objects)
    origin = Vector(((low.x + high.x) * 0.5, (low.y + high.y) * 0.5, low.z))
    scale = TITAN_HEIGHT_M / (high.z - low.z)
    body_transform = Matrix.Scale(scale, 4) @ Matrix.Translation(-origin)
    arm_transform = Matrix.Scale(scale, 4) @ Matrix.Translation(-shoulder_world)
    for obj in body:
        obj.data.transform(body_transform @ obj.matrix_world)
        obj.matrix_world = Matrix.Identity(4)
    for obj in right_arm:
        obj.data.transform(arm_transform @ obj.matrix_world)
        obj.matrix_world = Matrix.Identity(4)

    export_glb(body, TITAN_BODY_PATH)
    export_glb(right_arm, TITAN_ARM_PATH)
    body_receipt = output_receipt(TITAN_BODY_PATH, body)
    arm_receipt = output_receipt(TITAN_ARM_PATH, right_arm)

    shoulder_normalized = (shoulder_world - origin) * scale
    for obj in right_arm:
        obj.location = shoulder_normalized
    render_preview(body + right_arm, PREVIEW_DIR / "colossal-titan.png", portrait=True)
    for obj in right_arm:
        obj.location = Vector()
    shoulder_runtime = [shoulder_normalized.x, shoulder_normalized.z, -shoulder_normalized.y]
    return body_receipt, arm_receipt, [round(value, 5) for value in shoulder_runtime]


def render_preview(objects, output, portrait):
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    low, high = world_bounds(objects)
    center = (low + high) * 0.5
    height = high.z - low.z
    width = high.x - low.x
    span = max(height, width * 1.12)

    bpy.ops.object.camera_add(
        location=(center.x + span * 0.27, center.y - span * 1.82, center.z + span * 0.08)
    )
    camera = bpy.context.object
    camera.name = "QA Camera"
    camera.data.lens = 58
    camera.rotation_euler = ((center - camera.location).to_track_quat("-Z", "Y")).to_euler()
    bpy.context.scene.camera = camera

    for location, energy, size in [
        ((center.x - span * 0.55, center.y - span * 0.72, center.z + span * 0.82), 1700, span),
        ((center.x + span * 0.65, center.y + span * 0.28, center.z + span * 0.35), 1150, span * 0.7),
    ]:
        bpy.ops.object.light_add(type="AREA", location=location)
        light = bpy.context.object
        light.data.energy = energy
        light.data.shape = "DISK"
        light.data.size = size
        light.rotation_euler = ((center - light.location).to_track_quat("-Z", "Y")).to_euler()

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 560 if portrait else 720
    scene.render.resolution_y = 720 if portrait else 560
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = False
    if scene.world is None:
        scene.world = bpy.data.worlds.new("World Law QA World")
    scene.world.color = (0.018, 0.022, 0.032)
    scene.view_settings.look = "AgX - Medium High Contrast"
    scene.render.filepath = str(output)
    bpy.ops.render.render(write_still=True)

    bpy.data.objects.remove(camera, do_unlink=True)
    for obj in list(bpy.context.scene.objects):
        if obj.type == "LIGHT":
            bpy.data.objects.remove(obj, do_unlink=True)


def main():
    args = arguments()
    require_source(args.eva_source, EVA_SOURCE_SHA256, "EVA Unit-01")
    require_source(args.titan_source, TITAN_SOURCE_SHA256, "Colossal Titan")
    eva = prepare_eva(args.eva_source)
    titan_body, titan_arm, shoulder = prepare_titan(args.titan_source)
    receipt = {
        "schema": "pocket-openworld-world-law-models-v1",
        "sources": {
            "eva-unit-01": {
                "title": "EVANGELION UNIT ONE",
                "creator": "BROWNCOAT",
                "page": "https://sketchfab.com/3d-models/evangelion-unit-one-07081cd3a70e494095271c43a591af81",
                "sketchfab_uid": "07081cd3a70e494095271c43a591af81",
                "license": "CC BY 4.0",
                "license_url": "https://creativecommons.org/licenses/by/4.0/",
                "source_page_credit": "https://byneet.fanbox.cc/",
                "objaverse_path": "glbs/000-017/07081cd3a70e494095271c43a591af81.glb",
                "sha256": EVA_SOURCE_SHA256,
            },
            "colossal-titan": {
                "title": "Colossal Titan",
                "creator": "Sidaivan",
                "page": "https://sketchfab.com/3d-models/colossal-titan-e031a57fd4bf411f8e893361676b4544",
                "sketchfab_uid": "e031a57fd4bf411f8e893361676b4544",
                "license": "CC BY 4.0",
                "license_url": "https://creativecommons.org/licenses/by/4.0/",
                "objaverse_path": "glbs/000-028/e031a57fd4bf411f8e893361676b4544.glb",
                "sha256": TITAN_SOURCE_SHA256,
            },
        },
        "processing": {
            "eva_height_m": EVA_HEIGHT_M,
            "titan_height_m": TITAN_HEIGHT_M,
            "titan_right_arm": "posed down, split by Mixamo arm weights, origin at shoulder",
            "titan_right_shoulder_runtime_m": shoulder,
        },
        "outputs": {
            "eva-unit-01": eva,
            "colossal-titan-body": titan_body,
            "colossal-titan-right-arm": titan_arm,
        },
    }
    RECEIPT_PATH.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(json.dumps(receipt, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
