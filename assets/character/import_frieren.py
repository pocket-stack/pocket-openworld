"""Prepare the locally downloaded BOOTH Frieren model for Pocket Openworld.

This script does not download or redistribute the source model. Run it with a
local copy obtained from https://booth.pm/ja/items/5469071. It adds the five
runtime animation clips, embeds the supplied texture in a GLB, and writes
visual QA previews plus a small machine-readable receipt.

Example:
  blender --background --factory-startup --python import_frieren.py -- \
    --source "/Users/evan/Downloads/friren_1.1/frieren model.blend"
"""

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Quaternion, Vector


HERE = Path(__file__).resolve().parent
GLB_PATH = HERE / "frieren.glb"
PREVIEW_DIR = HERE / "frieren-previews"
RECEIPT_PATH = HERE / "frieren-receipt.json"
SOURCE_URL = "https://booth.pm/ja/items/5469071"


def arguments():
    parser = argparse.ArgumentParser(description="Prepare the local Frieren model")
    parser.add_argument("--source", type=Path, required=True)
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    return parser.parse_args(args)


def find_armature():
    armatures = [obj for obj in bpy.data.objects if obj.type == "ARMATURE"]
    if len(armatures) != 1:
        raise RuntimeError(f"expected one armature, found {[obj.name for obj in armatures]}")
    return armatures[0]


def character_meshes(armature):
    meshes = []
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        if any(mod.type == "ARMATURE" and mod.object == armature for mod in obj.modifiers):
            meshes.append(obj)
    if not meshes:
        raise RuntimeError("source model contains no meshes driven by its armature")
    return sorted(meshes, key=lambda obj: obj.name)


def validate_source(armature, meshes):
    required = {
        "Hips",
        "Spine",
        "Spine1",
        "Spine2",
        "Neck",
        "Head",
        "Arm.L",
        "foreArm.L",
        "hand.L",
        "Arm.R",
        "foreArm.R",
        "hand.R",
        "leg.L",
        "knee.L",
        "foot.L",
        "leg.R",
        "knee.R",
        "foot.R",
        "staff.R",
    }
    missing = sorted(required - set(armature.data.bones.keys()))
    if missing:
        raise RuntimeError(f"source rig is missing required bones: {missing}")
    weighted = {group.name for obj in meshes for group in obj.vertex_groups}
    if "hand.L" not in weighted or "staff.R" not in weighted:
        raise RuntimeError("source mesh lost its left-hand or staff skin weights")


def add_staff_tip_socket(armature, meshes):
    """Add a socket at the ornate end of the staff, in authored world units."""
    staff_meshes = [obj for obj in meshes if "staff" in obj.name.lower()]
    if len(staff_meshes) != 1:
        raise RuntimeError(
            f"expected one staff mesh, found {[obj.name for obj in staff_meshes]}"
        )
    staff = staff_meshes[0]
    hand = armature.matrix_world @ armature.data.bones["staff.R"].head_local
    points = [staff.matrix_world @ vertex.co for vertex in staff.data.vertices]
    farthest = max(points, key=lambda point: (point - hand).length)
    axis = (farthest - hand).normalized()
    furthest_projection = max((point - hand).dot(axis) for point in points)
    # Average the final 8 cm of geometry instead of using an outer ornament
    # vertex. The resulting point is the center of the visible casting head.
    tip_points = [
        point
        for point in points
        if (point - hand).dot(axis) >= furthest_projection - 0.08
    ]
    tip_world = sum(tip_points, Vector()) / len(tip_points)
    tip_local = armature.matrix_world.inverted() @ tip_world

    bpy.context.view_layer.objects.active = armature
    armature.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    old = armature.data.edit_bones.get("staff.tip")
    if old is not None:
        armature.data.edit_bones.remove(old)
    socket = armature.data.edit_bones.new("staff.tip")
    socket.head = tip_local
    socket.tail = tip_local + Vector((0.0, 2.0, 0.0))
    socket.parent = armature.data.edit_bones["staff.R"]
    socket.use_connect = False
    socket.use_deform = True
    bpy.ops.object.mode_set(mode="OBJECT")
    return tip_world


def normalize_materials(meshes):
    images = [image for image in bpy.data.images if image.size[0] > 0 and image.size[1] > 0]
    if len(images) != 1:
        raise RuntimeError(f"expected one source texture, found {[image.name for image in images]}")
    image = images[0]
    image.colorspace_settings.name = "sRGB"
    used = {material for obj in meshes for material in obj.data.materials if material is not None}
    if not used:
        raise RuntimeError("source meshes have no material")
    for material in used:
        material.use_nodes = True
        nodes = material.node_tree.nodes
        nodes.clear()
        output = nodes.new("ShaderNodeOutputMaterial")
        shader = nodes.new("ShaderNodeBsdfPrincipled")
        texture = nodes.new("ShaderNodeTexImage")
        texture.image = image
        shader.inputs["Base Color"].default_value = (1.0, 1.0, 1.0, 1.0)
        shader.inputs["Metallic"].default_value = 0.0
        shader.inputs["Roughness"].default_value = 0.92
        material.node_tree.links.new(texture.outputs["Color"], shader.inputs["Base Color"])
        material.node_tree.links.new(shader.outputs["BSDF"], output.inputs["Surface"])
        material.diffuse_color = (1.0, 1.0, 1.0, 1.0)


def clear_animation(armature):
    if armature.animation_data:
        armature.animation_data_clear()
    for action in list(bpy.data.actions):
        bpy.data.actions.remove(action)
    armature.animation_data_create()
    for bone in armature.pose.bones:
        bone.rotation_mode = "XYZ"


def reset_pose(armature):
    for bone in armature.pose.bones:
        bone.rotation_mode = "XYZ"
        bone.rotation_euler = (0.0, 0.0, 0.0)
        bone.location = (0.0, 0.0, 0.0)
        bone.scale = (1.0, 1.0, 1.0)


def begin_action(armature, name, loop):
    action = bpy.data.actions.new(name=name)
    action.use_fake_user = True
    action["loop"] = loop
    armature.animation_data.action = action
    return action


def aim_staff(armature, direction_world):
    bpy.context.view_layer.update()
    staff = armature.pose.bones["staff.R"]
    tip = armature.pose.bones["staff.tip"]
    grip_local = staff.head.copy()
    current_local = (tip.head - grip_local).normalized()
    desired_local = (
        armature.matrix_world.to_3x3().inverted() @ Vector(direction_world)
    ).normalized()
    correction = current_local.rotation_difference(desired_local)
    staff.matrix = (
        Matrix.Translation(grip_local)
        @ correction.to_matrix().to_4x4()
        @ Matrix.Translation(-grip_local)
        @ staff.matrix
    )
    bpy.context.view_layer.update()


def key_pose(armature, frame, rotations=None, locations=None, staff_direction=None):
    bpy.context.scene.frame_set(frame)
    reset_pose(armature)
    rotations = rotations or {}
    locations = locations or {}
    for name, value in rotations.items():
        armature.pose.bones[name].rotation_euler = value
    for name, value in locations.items():
        armature.pose.bones[name].location = value
    if staff_direction is not None:
        aim_staff(armature, staff_direction)
    for bone in armature.pose.bones:
        bone.keyframe_insert("rotation_euler", frame=frame, group=bone.name)
        bone.keyframe_insert("location", frame=frame, group=bone.name)
        bone.keyframe_insert("scale", frame=frame, group=bone.name)


def relaxed_pose(breath=0.0):
    # The downloaded model is authored in a T pose. Local-Z folds both arms
    # down alongside the body; the remaining values add a natural elbow bend.
    return {
        "Spine2": (0.0, breath * 0.35, 0.0),
        "Head": (breath * -0.18, 0.0, breath * 0.12),
        "Arm.L": (0.0, 0.04, math.radians(-72.0) + breath),
        "foreArm.L": (0.0, -0.08, math.radians(-8.0)),
        "hand.L": (0.0, 0.0, math.radians(-5.0)),
        "Arm.R": (0.0, -0.04, math.radians(60.0) - breath),
        "foreArm.R": (0.0, 0.10, math.radians(12.0)),
        "hand.R": (0.0, 0.0, math.radians(7.0)),
        "staff.R": (0.0, 0.0, math.radians(-4.0)),
    }


def merged(base, extra):
    result = dict(base)
    for name, delta in extra.items():
        original = result.get(name, (0.0, 0.0, 0.0))
        result[name] = tuple(original[i] + delta[i] for i in range(3))
    return result


def world_axis_rotation(armature, bone_name, world_axis, angle):
    """Return a pose-local Euler that hinges a bone around a world-space axis."""
    bone = armature.data.bones[bone_name]
    rest_basis = armature.matrix_world.to_3x3() @ bone.matrix_local.to_3x3()
    local_axis = rest_basis.inverted() @ Vector(world_axis)
    return Quaternion(local_axis.normalized(), angle).to_euler("XYZ")


def resting_pose(armature, breath=0.0):
    """Relaxed at-ease upper body with the staff clear of the skirt."""
    return relaxed_pose(breath)


def create_actions(armature):
    idle = begin_action(armature, "Idle", True)
    for frame, breath in ((1, -0.018), (24, 0.024), (48, -0.018)):
        stance = {
            "leg.L": world_axis_rotation(armature, "leg.L", (1, 0, 0), 0.05),
            "knee.L": world_axis_rotation(armature, "knee.L", (1, 0, 0), 0.08),
            "foot.L": world_axis_rotation(armature, "foot.L", (1, 0, 0), -0.13),
            "leg.R": world_axis_rotation(armature, "leg.R", (1, 0, 0), -0.12),
            "knee.R": world_axis_rotation(armature, "knee.R", (1, 0, 0), 0.16),
            "foot.R": world_axis_rotation(armature, "foot.R", (1, 0, 0), -0.04),
            "Spine": (0.0, breath * 0.18, math.radians(-1.4)),
            "Spine1": (0.0, 0.0, math.radians(1.0)),
        }
        key_pose(
            armature,
            frame,
            merged(resting_pose(armature, breath), stance),
            {
                "Hips": (
                    0.7,
                    0.35 + (0.45 if frame == 24 else 0.0),
                    -0.35,
                )
            },
            staff_direction=(-0.12, -0.76, -0.64),
        )

    walk = begin_action(armature, "Walk", True)
    # Every limb hinges around Blender world X, the character's anatomical
    # left-right axis. The source bones have mirrored/non-zero rolls, so
    # writing a raw local-X Euler makes the feet sweep sideways.
    walk_frames = (
        # frame, hip L/R, knee L/R, arm L/R, root vertical offset (cm)
        (1, -0.38, 0.34, 0.10, 0.20, 0.30, -0.30, -2.0),
        (9, 0.10, -0.05, 0.08, 0.62, -0.08, 0.08, 1.5),
        (17, 0.34, -0.38, 0.20, 0.10, -0.30, 0.30, -2.0),
        (25, -0.05, 0.10, 0.62, 0.08, 0.08, -0.08, 1.5),
        (33, -0.38, 0.34, 0.10, 0.20, 0.30, -0.30, -2.0),
    )
    for frame, left_leg, right_leg, left_knee, right_knee, left_arm, right_arm, bob in walk_frames:
        rotations = merged(
            resting_pose(armature, 0.0),
            {
                "leg.L": world_axis_rotation(armature, "leg.L", (1, 0, 0), left_leg),
                "knee.L": world_axis_rotation(armature, "knee.L", (1, 0, 0), left_knee),
                "foot.L": world_axis_rotation(
                    armature, "foot.L", (1, 0, 0), -(left_leg + left_knee)
                ),
                "leg.R": world_axis_rotation(armature, "leg.R", (1, 0, 0), right_leg),
                "knee.R": world_axis_rotation(armature, "knee.R", (1, 0, 0), right_knee),
                "foot.R": world_axis_rotation(
                    armature, "foot.R", (1, 0, 0), -(right_leg + right_knee)
                ),
                "Arm.L": world_axis_rotation(armature, "Arm.L", (1, 0, 0), left_arm),
                "Arm.R": world_axis_rotation(armature, "Arm.R", (1, 0, 0), right_arm),
                "Spine2": (0.0, 0.0, (left_leg - right_leg) * 0.025),
            },
        )
        # Hips local Y maps to world vertical. A 3.5 cm excursion makes weight
        # transfer readable while keeping the authored foot plane grounded.
        key_pose(
            armature,
            frame,
            rotations,
            {"Hips": (0.0, bob, 0.0)},
            staff_direction=(-0.12, -0.76, -0.64),
        )

    chop = begin_action(armature, "Chop", False)
    chop_frames = {
        1: ({}, {}),
        8: (
            {
                "Spine2": (-0.12, 0.08, -0.18),
                "Arm.R": (-0.95, -0.15, -0.20),
                "foreArm.R": (-0.58, 0.06, -0.18),
                "hand.R": (-0.18, 0.0, -0.12),
                "staff.R": (-0.22, 0.08, -0.10),
                "Arm.L": (0.25, 0.0, -0.10),
            },
            {"Hips": (0.0, 0.0, 1.0)},
        ),
        14: (
            {
                "Spine2": (-0.18, 0.12, -0.25),
                "Arm.R": (-1.25, -0.18, -0.28),
                "foreArm.R": (-0.78, 0.08, -0.22),
                "hand.R": (-0.24, 0.0, -0.18),
                "staff.R": (-0.34, 0.10, -0.15),
                "Arm.L": (0.34, 0.0, -0.14),
            },
            {"Hips": (0.0, 0.0, 1.7)},
        ),
        22: (
            {
                "Spine2": (0.36, -0.10, 0.28),
                "Arm.R": (0.82, 0.12, 0.28),
                "foreArm.R": (0.48, -0.05, 0.18),
                "hand.R": (0.18, 0.0, 0.12),
                "staff.R": (0.24, -0.08, 0.12),
                "Arm.L": (-0.30, 0.0, 0.12),
            },
            {"Hips": (0.0, 0.0, -1.4)},
        ),
        28: (
            {
                "Spine2": (0.20, -0.05, 0.14),
                "Arm.R": (0.42, 0.06, 0.12),
                "foreArm.R": (0.25, 0.0, 0.08),
                "Arm.L": (-0.14, 0.0, 0.05),
            },
            {"Hips": (0.0, 0.0, -0.7)},
        ),
        36: ({}, {}),
    }
    for frame, (rotations, locations) in chop_frames.items():
        key_pose(armature, frame, merged(relaxed_pose(0.0), rotations), locations)

    cast = begin_action(armature, "Cast", False)
    cast_frames = {
        1: ({}, {}),
        6: (
            {
                "Spine2": (-0.06, 0.0, -0.12),
                "Arm.R": (-0.32, -0.08, -0.78),
                "foreArm.R": (-0.30, 0.08, -0.28),
                "hand.R": (-0.12, 0.0, -0.08),
                "staff.R": (-0.16, 0.0, -0.10),
                "Arm.L": (0.24, -0.08, 0.62),
                "foreArm.L": (-0.36, -0.08, 0.34),
                "hand.L": (0.0, 0.30, -0.08),
            },
            {"Hips": (0.0, 0.0, 0.6)},
        ),
        12: (
            {
                "Spine2": (0.08, 0.0, 0.18),
                "Head": (-0.06, 0.0, -0.10),
                "Arm.R": (-0.58, -0.12, -1.08),
                "foreArm.R": (-0.52, 0.10, -0.42),
                "hand.R": (-0.18, 0.0, -0.12),
                "staff.R": (-0.30, 0.04, -0.16),
                "Arm.L": (0.42, -0.16, 0.94),
                "foreArm.L": (-0.62, -0.10, 0.48),
                "hand.L": (0.0, 0.55, -0.12),
            },
            {"Hips": (0.0, 0.0, 1.4)},
        ),
        18: (
            {
                "Spine2": (0.02, 0.0, 0.08),
                "Arm.R": (-0.42, -0.08, -0.86),
                "foreArm.R": (-0.36, 0.08, -0.34),
                "staff.R": (-0.20, 0.02, -0.12),
                "Arm.L": (0.30, -0.10, 0.72),
                "foreArm.L": (-0.44, -0.08, 0.38),
                "hand.L": (0.0, 0.36, -0.10),
            },
            {"Hips": (0.0, 0.0, 0.5)},
        ),
        24: ({}, {}),
    }
    for frame, (rotations, locations) in cast_frames.items():
        key_pose(armature, frame, merged(relaxed_pose(0.0), rotations), locations)

    water = begin_action(armature, "Water", False)
    water_frames = {
        1: ({}, {}),
        6: (
            {
                "Spine2": (0.08, 0.0, 0.0),
                "Arm.R": (0.0, 0.0, -0.55),
                "foreArm.R": (0.0, 0.0, -0.22),
                "staff.R": (0.0, 0.0, -0.04),
                "Arm.L": (-0.30, -0.10, 0.54),
                "foreArm.L": (-0.50, 0.0, 0.36),
                "hand.L": (0.0, 0.42, 0.0),
            },
            {"Hips": (0.0, -0.4, 0.0)},
        ),
        12: (
            {
                "Spine2": (0.12, 0.0, 0.0),
                "Head": (-0.04, 0.0, 0.0),
                "Arm.R": (0.0, 0.0, -0.78),
                "foreArm.R": (0.0, 0.0, -0.32),
                "staff.R": (0.0, 0.0, -0.06),
                "Arm.L": (-0.42, -0.14, 0.72),
                "foreArm.L": (-0.62, 0.0, 0.46),
                "hand.L": (0.0, 0.58, 0.0),
            },
            {"Hips": (0.0, -0.8, 0.0)},
        ),
        20: (
            {
                "Spine2": (0.08, 0.0, 0.0),
                "Arm.R": (0.0, 0.0, -0.68),
                "foreArm.R": (0.0, 0.0, -0.27),
                "staff.R": (0.0, 0.0, -0.05),
                "Arm.L": (-0.36, -0.12, 0.62),
                "foreArm.L": (-0.56, 0.0, 0.40),
                "hand.L": (0.0, 0.50, 0.0),
            },
            {"Hips": (0.0, -0.5, 0.0)},
        ),
        28: ({}, {}),
    }
    for frame, (rotations, locations) in water_frames.items():
        key_pose(armature, frame, merged(relaxed_pose(0.0), rotations), locations)

    armature.animation_data.action = idle
    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = 48
    bpy.context.scene.render.fps = 30
    bpy.context.scene.frame_set(1)
    return {"Idle": idle, "Walk": walk, "Chop": chop, "Cast": cast, "Water": water}


def motion_metrics(armature, actions):
    """Measure animation axes in Blender world space (X side, -Y forward, Z up)."""
    scene = bpy.context.scene

    def point(name):
        return armature.matrix_world @ armature.pose.bones[name].head

    armature.animation_data.action = actions["Walk"]
    samples = []
    for frame in range(1, 34):
        scene.frame_set(frame)
        samples.append(
            {
                "frame": frame,
                "hips": point("Hips"),
                "left_foot": point("foot.L"),
                "right_foot": point("foot.R"),
            }
        )

    def axis_range(key, axis):
        values = [sample[key][axis] for sample in samples]
        return max(values) - min(values)

    result = {
        "coordinate_system": {"lateral": "+X", "forward": "-Y", "up": "+Z"},
        "left_foot_lateral_range_m": axis_range("left_foot", 0),
        "left_foot_forward_range_m": axis_range("left_foot", 1),
        "right_foot_lateral_range_m": axis_range("right_foot", 0),
        "right_foot_forward_range_m": axis_range("right_foot", 1),
        "hips_vertical_range_m": axis_range("hips", 2),
    }
    if max(result["left_foot_lateral_range_m"], result["right_foot_lateral_range_m"]) > 0.02:
        raise RuntimeError(f"walk feet sweep sideways: {result}")
    if min(result["left_foot_forward_range_m"], result["right_foot_forward_range_m"]) < 0.40:
        raise RuntimeError(f"walk stride has no readable fore-aft travel: {result}")
    if not 0.025 <= result["hips_vertical_range_m"] <= 0.055:
        raise RuntimeError(f"walk center-of-mass excursion is implausible: {result}")

    armature.animation_data.action = actions["Idle"]
    scene.frame_set(24)
    idle_left_foot = point("foot.L")
    idle_right_foot = point("foot.R")
    idle_staff_grip = point("staff.R")
    idle_staff_tip = point("staff.tip")
    idle_staff_direction = (idle_staff_tip - idle_staff_grip).normalized()
    result.update(
        {
            "idle_foot_stagger_m": abs(idle_left_foot.y - idle_right_foot.y),
            "idle_staff_grip_lateral_m": abs(idle_staff_grip.x),
            "idle_staff_tip_height_m": idle_staff_tip.z,
            "idle_staff_vertical_slope": idle_staff_direction.z,
        }
    )
    if result["idle_foot_stagger_m"] < 0.07:
        raise RuntimeError(f"Idle pose returned to a rigid attention stance: {result}")
    if result["idle_staff_grip_lateral_m"] < 0.20:
        raise RuntimeError(f"Idle staff grip moved back into the skirt silhouette: {result}")
    if not 0.02 <= result["idle_staff_tip_height_m"] <= 0.15:
        raise RuntimeError(f"Idle staff no longer rests just above the ground: {result}")
    if result["idle_staff_vertical_slope"] > -0.50:
        raise RuntimeError(f"Idle staff is not angled down outside the skirt: {result}")

    armature.animation_data.action = actions["Water"]
    scene.frame_set(12)
    staff_grip = point("staff.R")
    staff_tip = point("staff.tip")
    staff_direction = (staff_tip - staff_grip).normalized()
    result.update(
        {
            "water_staff_tip_height_m": staff_tip.z,
            "water_staff_forward_alignment": -staff_direction.y,
            "water_staff_vertical_slope": staff_direction.z,
        }
    )
    if result["water_staff_forward_alignment"] < 0.95:
        raise RuntimeError(f"Water clip does not point the staff forward: {result}")
    return {key: round(value, 5) if isinstance(value, float) else value for key, value in result.items()}


def export_glb(armature, meshes):
    bpy.ops.object.select_all(action="DESELECT")
    armature.select_set(True)
    for obj in meshes:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = armature
    result = bpy.ops.export_scene.gltf(
        filepath=str(GLB_PATH),
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_materials="EXPORT",
        export_animations=True,
        export_animation_mode="ACTIONS",
        export_force_sampling=True,
        export_frame_step=1,
        export_skins=True,
        export_all_influences=False,
        export_influence_nb=4,
        export_def_bones=True,
        export_leaf_bone=False,
        export_optimize_animation_size=True,
        export_optimize_animation_keep_anim_armature=True,
        export_armature_object_remove=False,
        export_extras=True,
        export_cameras=False,
        export_lights=False,
        export_current_frame=False,
        export_apply=False,
    )
    if "FINISHED" not in result:
        raise RuntimeError(f"glTF export failed: {result}")


def look_at(obj, target):
    obj.rotation_euler = (target - obj.location).to_track_quat("-Z", "Y").to_euler()


def material(name, color):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    return mat


def render_previews(armature, meshes, actions):
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    # The downloaded file is configured for FFmpeg output and Blender 5.1
    # does not allow changing that legacy scene to a still-image format.
    # Render QA in a clean scene while sharing the character objects/actions.
    scene = bpy.data.scenes.new("Frieren QA")
    bpy.context.window.scene = scene
    scene.collection.objects.link(armature)
    for obj in meshes:
        scene.collection.objects.link(obj)
    bpy.ops.mesh.primitive_plane_add(size=20, location=(0.0, 0.0, -0.015))
    floor = bpy.context.object
    floor.name = "QA Floor"
    floor.data.materials.append(material("QA Floor", (0.055, 0.08, 0.075)))

    bpy.ops.object.camera_add()
    camera = bpy.context.object
    bpy.context.scene.camera = camera
    camera.data.lens = 58

    for location, energy, size in (
        ((3.2, -4.8, 3.4), 900, 4.0),
        ((-3.0, -2.0, 2.2), 650, 3.0),
        ((0.0, 3.2, 4.0), 850, 3.5),
    ):
        data = bpy.data.lights.new("QA Area", "AREA")
        data.energy = energy
        data.shape = "DISK"
        data.size = size
        light = bpy.data.objects.new("QA Area", data)
        bpy.context.collection.objects.link(light)
        light.location = location
        look_at(light, Vector((0.0, 0.0, 0.9)))

    world = bpy.data.worlds.new("QA World")
    scene.world = world
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.018, 0.025, 0.024, 1.0)
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.32

    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 768
    scene.render.resolution_y = 768
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.view_settings.look = "AgX - Medium High Contrast"

    views = (
        ("idle-front", "Idle", 24, (3.0, -5.1, 2.45)),
        ("idle-rear", "Idle", 24, (0.0, 5.4, 2.15)),
        ("walk-side", "Walk", 1, (5.4, 0.0, 2.15)),
        ("chop", "Chop", 22, (3.0, -5.1, 2.45)),
        ("cast", "Cast", 12, (3.0, -5.1, 2.45)),
        ("water", "Water", 12, (3.0, -5.1, 2.45)),
    )
    for name, action_name, frame, position in views:
        armature.animation_data.action = actions[action_name]
        scene.frame_set(frame)
        camera.location = position
        look_at(camera, Vector((0.0, 0.0, 0.9)))
        scene.render.filepath = str(PREVIEW_DIR / f"{name}.png")
        bpy.ops.render.render(write_still=True)


def write_receipt(source, armature, meshes, actions, staff_tip):
    receipt = {
        "schema": "pocket-openworld-local-character-v1",
        "source": {
            "page": SOURCE_URL,
            "local_filename": source.name,
            "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        },
        "output": {
            "glb": GLB_PATH.name,
            "sha256": hashlib.sha256(GLB_PATH.read_bytes()).hexdigest(),
            "bytes": GLB_PATH.stat().st_size,
        },
        "rig": {
            "bones": len(armature.data.bones),
            "meshes": [obj.name for obj in meshes],
            "vertices": sum(len(obj.data.vertices) for obj in meshes),
            "triangles": sum(len(obj.data.loop_triangles) for obj in meshes),
            "required_socket": "hand.L",
            "tool_bone": "staff.R",
            "staff_tip_socket": "staff.tip",
            "staff_tip_world_m": [round(value, 5) for value in staff_tip],
        },
        "motion": motion_metrics(armature, actions),
        "animations": {
            name: {"frames": list(action.frame_range), "loop": bool(action.get("loop", False))}
            for name, action in actions.items()
        },
    }
    RECEIPT_PATH.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print("POCKET_OPENWORLD_FRIEREN_OK", json.dumps(receipt, sort_keys=True))


def main():
    args = arguments()
    source = args.source.expanduser().resolve()
    if not source.is_file():
        raise RuntimeError(f"source model does not exist: {source}")
    bpy.ops.wm.open_mainfile(filepath=str(source))
    armature = find_armature()
    meshes = character_meshes(armature)
    validate_source(armature, meshes)
    staff_tip = add_staff_tip_socket(armature, meshes)
    normalize_materials(meshes)
    clear_animation(armature)
    actions = create_actions(armature)
    export_glb(armature, meshes)
    render_previews(armature, meshes, actions)
    write_receipt(source, armature, meshes, actions, staff_tip)


if __name__ == "__main__":
    main()
