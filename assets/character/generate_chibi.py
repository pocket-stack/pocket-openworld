#!/usr/bin/env python3
"""Clean-room chibi Frieren: authored geometry, rig, materials and seven clips.

No downloaded blend/FBX/GLB, texture, mesh, rig or animation is read. The only
shared code is this repository's original primitive/studio helper module.
Blender 5.1: --background --factory-startup --python assets/character/generate_chibi.py
"""

import argparse
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import struct
import sys
import bpy
from mathutils import Matrix, Vector

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "primitives", HERE / "generate_character.py"
)
g = importlib.util.module_from_spec(spec)
spec.loader.exec_module(g)
parser = argparse.ArgumentParser()
parser.add_argument("--output-dir", type=Path, default=HERE)
parser.add_argument("--skip-previews", action="store_true")
args = parser.parse_args(
    sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
)
OUT = args.output_dir.resolve()
OUT.mkdir(parents=True, exist_ok=True)
g.reset_scene()

# Z-up, -Y facing, soles at Z=0; glTF becomes Y-up, +Z facing.
data = bpy.data.armatures.new("ChibiRig")
rig = bpy.data.objects.new("ChibiRig", data)
bpy.context.collection.objects.link(rig)
bpy.context.view_layer.objects.active = rig
rig.select_set(True)
bpy.ops.object.mode_set(mode="EDIT")
bones = {
    "root": ((0, 0, 0), (0, 0, 0.15), None),
    "Hips": ((0, 0, 0.49), (0, 0, 0.64), "root"),
    "Spine": ((0, 0, 0.64), (0, 0, 0.78), "Hips"),
    "Spine1": ((0, 0, 0.78), (0, 0, 0.91), "Spine"),
    "Spine2": ((0, 0, 0.91), (0, 0, 1.04), "Spine1"),
    "Neck": ((0, 0, 1.04), (0, 0, 1.13), "Spine2"),
    "Head": ((0, 0, 1.13), (0, 0, 1.69), "Neck"),
}
for side, s in [("L", 1), ("R", -1)]:
    bones.update(
        {
            f"Arm.{side}": ((s * 0.235, 0, 0.95), (s * 0.30, 0, 0.75), "Spine2"),
            f"foreArm.{side}": (
                (s * 0.30, 0, 0.75),
                (s * 0.34, -0.015, 0.57),
                f"Arm.{side}",
            ),
            f"hand.{side}": (
                (s * 0.34, -0.015, 0.57),
                (s * 0.34, -0.02, 0.49),
                f"foreArm.{side}",
            ),
            f"leg.{side}": ((s * 0.125, 0, 0.51), (s * 0.125, 0, 0.29), "Hips"),
            f"knee.{side}": (
                (s * 0.125, 0, 0.29),
                (s * 0.125, 0, 0.105),
                f"leg.{side}",
            ),
            f"foot.{side}": (
                (s * 0.125, 0, 0.105),
                (s * 0.125, -0.12, 0.07),
                f"knee.{side}",
            ),
            f"hair.{side}": ((s * 0.29, 0.035, 1.50), (s * 0.40, 0.09, 0.91), "Head"),
        }
    )
bones["staff.R"] = ((-0.37, -0.07, 0.54), (-0.37, -0.07, 0.84), "hand.R")
bones["staff.tip"] = ((-0.37, -0.07, 1.48), (-0.37, -0.07, 1.56), "staff.R")
for name, (head, tail, parent) in bones.items():
    bone = data.edit_bones.new(name)
    bone.head, bone.tail = head, tail
    if parent:
        bone.parent = data.edit_bones[parent]
    bone.align_roll(Vector((0, 1, 0)))
bpy.ops.object.mode_set(mode="OBJECT")
rig.select_set(False)
g.ARMATURE = rig
rig["provenance"] = (
    "Clean-room procedural geometry and authored actions; no external model inputs"
)
rig["runtime_contract"] = (
    "Idle Walk Chop Cast Water Climb Fall; hand.L staff.R staff.tip"
)

m = {
    "skin": g.mat("Porcelain peach", (0.93, 0.66, 0.49)),
    "cream": g.mat("Ivory wool", (0.88, 0.85, 0.70)),
    "white": g.mat("Silver hair", (0.68, 0.73, 0.79)),
    "shade": g.mat("Silver hair shadow", (0.40, 0.47, 0.56)),
    "dark": g.mat("Ink plum", (0.045, 0.031, 0.055)),
    "gold": g.mat("Warm gold", (0.62, 0.34, 0.07), 0.5, 0.22),
    "green": g.mat("Jade eyes", (0.075, 0.35, 0.23), 0.45),
    "red": g.mat("Garnet", (0.43, 0.018, 0.046), 0.35),
    "wood": g.mat("Walnut staff", (0.20, 0.071, 0.035)),
    "glint": g.mat("Eye highlights", (0.98, 0.99, 1.0), 0.35),
}


def sphere(name, p, s, material, bone, segments=16, rings=10):
    return g.uv_sphere(name, p, s, m[material], bone, segments, rings)


def rod(name, a, b, r, material, bone, vertices=10):
    return g.cylinder_between(name, a, b, r, m[material], bone, vertices)


def curve(name, points, r, material, bone):
    # A polyline tube with an explicit deterministic ring frame.
    verts = []
    faces = []
    sides = 8
    for i, p in enumerate(points):
        p = Vector(p)
        t = Vector(points[min(i + 1, len(points) - 1)]) - Vector(points[max(i - 1, 0)])
        t.normalize()
        u = t.cross(Vector((0, 1, 0))).normalized()
        v = t.cross(u).normalized()
        for j in range(sides):
            a = j * math.tau / sides
            verts.append(p + r * (math.cos(a) * u + math.sin(a) * v))
    for i in range(len(points) - 1):
        for j in range(sides):
            a = i * sides + j
            b = i * sides + (j + 1) % sides
            faces.append((a, b, b + sides, a + sides))
    faces += [
        tuple(reversed(range(sides))),
        tuple(range((len(points) - 1) * sides, len(points) * sides)),
    ]
    return g.custom_mesh(name, verts, faces, m[material], bone, smooth=True)


def tuft(name, points, widths, depths, bone, material="white"):
    verts = []
    faces = []
    n = 8
    for p, w, d in zip(points, widths, depths):
        for j in range(n):
            a = j * math.tau / n
            verts.append((p[0] + math.cos(a) * w, p[1] + math.sin(a) * d, p[2]))
    for i in range(len(points) - 1):
        for j in range(n):
            faces.append(
                (
                    i * n + j,
                    i * n + (j + 1) % n,
                    (i + 1) * n + (j + 1) % n,
                    (i + 1) * n + j,
                )
            )
    faces += [
        tuple(reversed(range(n))),
        tuple(range((len(points) - 1) * n, len(points) * n)),
    ]
    return g.custom_mesh(name, verts, faces, m[material], bone, smooth=True)


# Black stockings and rounded boots, white/gold flared tunic.
for side, s in [("L", 1), ("R", -1)]:
    g.cone_between(
        "Thigh " + side,
        (s * 0.125, 0, 0.29),
        (s * 0.125, 0, 0.50),
        0.08,
        0.095,
        m["dark"],
        f"leg.{side}",
        12,
    )
    g.cone_between(
        "Shin " + side,
        (s * 0.125, 0, 0.105),
        (s * 0.125, 0, 0.29),
        0.068,
        0.08,
        m["dark"],
        f"knee.{side}",
        12,
    )
    sphere(
        "Knee " + side,
        (s * 0.125, 0, 0.29),
        (0.081, 0.081, 0.07),
        "dark",
        f"knee.{side}",
        12,
        8,
    )
    boot = g.cube(
        "Boot " + side,
        (s * 0.125, -0.046, 0.064),
        (0.18, 0.25, 0.128),
        m["dark"],
        f"foot.{side}",
        bevel=0.14,
    )
    boot["sole"] = True
    rod(
        "Boot trim " + side,
        (s * 0.125 - 0.07, -0.04, 0.147),
        (s * 0.125 + 0.07, -0.04, 0.147),
        0.015,
        "gold",
        f"foot.{side}",
    )
# Ring geometry is softer than a cone and forms a continuous garment silhouette.
tuft(
    "White tunic",
    [(0, 0, 0.42), (0, 0, 0.47), (0, 0, 0.74), (0, 0, 0.99)],
    [0.30, 0.31, 0.23, 0.23],
    [0.19, 0.20, 0.145, 0.145],
    "Hips",
    "cream",
)
# The torso follows the spine while the skirt follows the hips.
sphere("Bodice", (0, 0, 0.85), (0.231, 0.152, 0.22), "cream", "Spine1")
curve(
    "Hem gold",
    [
        (-0.285, -0.073, 0.443),
        (-0.21, -0.158, 0.443),
        (0, -0.192, 0.443),
        (0.21, -0.158, 0.443),
        (0.285, -0.073, 0.443),
    ],
    0.016,
    "gold",
    "Hips",
)
curve(
    "Coat front seam",
    [(0, -0.157, 0.95), (0, -0.155, 0.78), (0, -0.179, 0.60), (0, -0.196, 0.46)],
    0.012,
    "gold",
    "Hips",
)
# Cape shoulder yoke and black high collar.
sphere("Collar", (0, 0, 1.033), (0.12, 0.12, 0.09), "dark", "Neck")
for side, s in [("L", 1), ("R", -1)]:
    sphere(
        "Cape shoulder " + side,
        (s * 0.17, 0.034, 0.965),
        (0.17, 0.19, 0.115),
        "cream",
        "Spine2",
    )
    curve(
        "Cape piping " + side,
        [
            (0, -0.135, 0.985),
            (s * 0.18, -0.144, 0.916),
            (s * 0.30, -0.075, 0.91),
            (s * 0.28, 0.16, 0.90),
        ],
        0.014,
        "gold",
        "Spine2",
    )
    g.cone_between(
        "Upper sleeve " + side,
        (s * 0.237, 0, 0.94),
        (s * 0.30, 0, 0.745),
        0.105,
        0.085,
        m["cream"],
        f"Arm.{side}",
        12,
    )
    g.cone_between(
        "Bell sleeve " + side,
        (s * 0.30, 0, 0.76),
        (s * 0.34, -0.015, 0.58),
        0.085,
        0.10,
        m["cream"],
        f"foreArm.{side}",
        12,
    )
    rod(
        "Cuff " + side,
        (s * 0.337, -0.014, 0.606),
        (s * 0.344, -0.016, 0.577),
        0.102,
        "gold",
        f"foreArm.{side}",
        12,
    )
    sphere(
        "Hand " + side,
        (s * 0.344, -0.026, 0.535),
        (0.068, 0.067, 0.075),
        "skin",
        f"hand.{side}",
        12,
        8,
    )
sphere(
    "Collar garnet", (0, -0.14, 0.971), (0.035, 0.016, 0.046), "red", "Spine2", 12, 8
)

# Oversized head with a readable face; original mesh, no image texture.
sphere("Face", (0, -0.012, 1.375), (0.345, 0.285, 0.345), "skin", "Head", 24, 16)
sphere("Hair cap", (0, 0.025, 1.474), (0.355, 0.275, 0.30), "white", "Head", 24, 12)
sphere("Back hair", (0, 0.105, 1.38), (0.348, 0.24, 0.327), "white", "Head", 24, 12)
# Pointed elf ears, with inset warmer inner triangles.
for side, s in [("L", 1), ("R", -1)]:
    g.custom_mesh(
        "Elf ear " + side,
        [
            (s * 0.28, -0.025, 1.34),
            (s * 0.56, 0.015, 1.425),
            (s * 0.325, 0.04, 1.22),
            (s * 0.35, 0.07, 1.34),
        ],
        [(0, 1, 2), (2, 1, 3), (0, 3, 1), (0, 2, 3)],
        m["skin"],
        "Head",
        smooth=True,
    )
    g.custom_mesh(
        "Inner ear " + side,
        [(s * 0.328, -0.027, 1.335), (s * 0.492, 0.002, 1.395), (s * 0.35, 0.0, 1.275)],
        [(0, 1, 2)],
        m["gold"],
        "Head",
    )
    rod(
        "Earring " + side,
        (s * 0.37, -0.012, 1.29),
        (s * 0.37, -0.012, 1.23),
        0.013,
        "gold",
        "Head",
    )
    sphere(
        "Earring stone " + side,
        (s * 0.37, -0.012, 1.205),
        (0.024, 0.022, 0.034),
        "red",
        "Head",
        10,
        6,
    )
    # Twin tails fan away from the face and taper down beside the cape.
    sphere(
        "Hair tie " + side,
        (s * 0.315, 0.068, 1.50),
        (0.06, 0.075, 0.067),
        "gold",
        "Head",
        12,
        8,
    )
    for j in range(3):
        x = s * (0.34 + j * 0.03)
        y = 0.09 + (j - 1) * 0.045
        tuft(
            "Twin tail " + side + str(j),
            [
                (x, y, 1.50),
                (s * (0.43 + j * 0.027), y + 0.035, 1.25),
                (s * (0.43 + j * 0.025), y + 0.06, 0.99),
                (s * (0.38 + j * 0.027), y + 0.09, 0.83),
            ],
            [0.06, 0.069, 0.055, 0.008],
            [0.07, 0.078, 0.055, 0.008],
            f"hair.{side}",
        )
    # Side locks keep the distinctive straight fringe and long framing strands.
    tuft(
        "Side lock " + side,
        [(s * 0.29, -0.10, 1.65), (s * 0.32, -0.16, 1.43), (s * 0.31, -0.20, 1.18)],
        [0.075, 0.064, 0.005],
        [0.06, 0.06, 0.006],
        "Head",
    )
# Individually tapered fringe, leaving green eyes visible below it.
for j in range(7):
    x = (j - 3) * 0.076
    tuft(
        "Fringe " + str(j),
        [
            (x * 0.82, -0.12, 1.72),
            (x, -0.239, 1.58),
            (x * 0.94, -0.285, 1.45 + (0.025 if j % 2 else 0)),
        ],
        [0.072, 0.052, 0.003],
        [0.075, 0.042, 0.004],
        "Head",
    )
# Slightly sleepy, wide jade eyes, upper lashes and brows.
for s in [-1, 1]:
    x = s * 0.124
    sphere(
        "Eye white " + str(s),
        (x, -0.272, 1.367),
        (0.087, 0.024, 0.068),
        "glint",
        "Head",
        16,
        10,
    )
    sphere(
        "Jade iris " + str(s),
        (x, -0.296, 1.365),
        (0.040, 0.010, 0.052),
        "green",
        "Head",
        16,
        10,
    )
    sphere(
        "Pupil " + str(s),
        (x, -0.305, 1.367),
        (0.020, 0.006, 0.039),
        "dark",
        "Head",
        12,
        8,
    )
    sphere(
        "Eye glint " + str(s),
        (x - 0.011, -0.312, 1.388),
        (0.011, 0.003, 0.014),
        "glint",
        "Head",
        10,
        6,
    )
    curve(
        "Upper lash " + str(s),
        [(x - 0.081, -0.283, 1.396), (x, -0.302, 1.410), (x + 0.081, -0.283, 1.398)],
        0.010,
        "dark",
        "Head",
    )
    curve(
        "Brow " + str(s),
        [(x - 0.060, -0.277, 1.438), (x, -0.286, 1.447), (x + 0.062, -0.277, 1.441)],
        0.006,
        "shade",
        "Head",
    )
sphere("Nose", (0, -0.302, 1.29), (0.023, 0.023, 0.017), "skin", "Head", 10, 6)
curve(
    "Quiet smile",
    [(-0.027, -0.271, 1.228), (0, -0.279, 1.222), (0.027, -0.271, 1.228)],
    0.005,
    "dark",
    "Head",
)

# Staff has its own mesh and deform bone; the outlet follows the red gemstone.
body_objects = list(g.CHARACTER_OBJECTS)
g.CHARACTER_OBJECTS.clear()
rod(
    "Staff shaft",
    (-0.37, -0.07, 0.065),
    (-0.37, -0.07, 1.40),
    0.021,
    "wood",
    "staff.R",
    12,
)
rod(
    "Staff ferrule",
    (-0.37, -0.07, 0.035),
    (-0.37, -0.07, 0.115),
    0.025,
    "gold",
    "staff.R",
    12,
)
rod(
    "Staff grip",
    (-0.37, -0.07, 0.46),
    (-0.37, -0.07, 0.62),
    0.027,
    "gold",
    "staff.R",
    12,
)
curve(
    "Staff crook",
    [
        (-0.37, -0.07, 1.31),
        (-0.48, -0.07, 1.37),
        (-0.49, -0.07, 1.52),
        (-0.39, -0.07, 1.60),
        (-0.28, -0.07, 1.55),
        (-0.26, -0.07, 1.46),
    ],
    0.028,
    "gold",
    "staff.R",
)
sphere(
    "Staff ruby", (-0.37, -0.07, 1.48), (0.071, 0.057, 0.081), "red", "staff.R", 16, 10
)
staff_objects = list(g.CHARACTER_OBJECTS)


def join(objects, name):
    active = objects[0]
    for o in objects[1:]:
        bpy.ops.object.select_all(action="DESELECT")
        active.select_set(True)
        o.select_set(True)
        bpy.context.view_layer.objects.active = active
        bpy.ops.object.join()
    active.name = name
    names = [
        active.material_slots[p.material_index].material.name
        for p in active.data.polygons
    ]
    materials = sorted(
        {slot.material.name: slot.material for slot in active.material_slots}.items()
    )
    active.data.materials.clear()
    for _, mat in materials:
        active.data.materials.append(mat)
    lookup = {name: i for i, (name, _) in enumerate(materials)}
    for p, n in zip(active.data.polygons, names):
        p.material_index = lookup[n]
    for modifier in list(active.modifiers)[1:]:
        if modifier.type == "ARMATURE":
            active.modifiers.remove(modifier)
    return active


body = join(body_objects, "Chibi Frieren")
staff = join(staff_objects, "staff")
g.CHARACTER_OBJECTS[:] = [body, staff]

# Author every action in this file, never retarget or import source animation.
# Per-frame grounding uses the skinned boot vertices (not just ankle origins).
boot_groups = [body.vertex_groups.get("foot." + s).index for s in ["L", "R"]]
boot_indices = [
    v.index
    for v in body.data.vertices
    if any(x.group in boot_groups and x.weight > 0.5 for x in v.groups)
]
actions = {}
motion = {}
water_keys = []
lengths = {
    "Idle": 60,
    "Walk": 32,
    "Chop": 30,
    "Cast": 30,
    "Water": 30,
    "Climb": 40,
    "Fall": 30,
}
for name, count in lengths.items():
    action = g.action_begin(name)
    actions[name] = action
    samples = []
    for frame in range(1, count + 2):
        t = (frame - 1) / count
        phase = t * math.tau
        bpy.context.scene.frame_set(frame)
        g.reset_pose()

        def rot(b, x=0, y=0, z=0):
            rig.pose.bones[b].rotation_euler = (x, y, z)

        rot("Spine2", 0.025 * math.sin(phase))
        rot("Head", -0.015 * math.sin(phase), 0, 0.014 * math.sin(phase))
        rot("Arm.L", -0.08, 0, -0.045)
        rot("foreArm.L", -0.10)
        rot("Arm.R", -0.07, 0, 0.06)
        rot("foreArm.R", -0.12)
        for s, sign in [("L", 1), ("R", -1)]:
            rot(
                "hair." + s,
                0.025 * math.sin(phase + sign),
                0,
                0.025 * sign * math.sin(phase),
            )
        if name == "Walk":
            for s, sign in [("L", 1), ("R", -1)]:
                swing = math.cos(phase) * sign
                rot("leg." + s, 0.52 * swing)
                rot("knee." + s, -0.50 * max(0, -math.sin(phase) * sign))
                rot("foot." + s, -0.14 * swing)
                rot("Arm." + s, -0.34 * swing, 0, -0.04 * sign)
                rot("foreArm." + s, -0.20)
            rot(
                "Spine", -0.075 - 0.02 * math.cos(phase * 2), 0, 0.025 * math.sin(phase)
            )
            rot("Spine2", 0, 0.07 * math.cos(phase))
            rot("Head", 0.04, 0, -0.02 * math.sin(phase))
        elif name in ("Chop", "Cast", "Water"):
            pulse = math.sin(math.pi * t) ** 2
            rot("Arm.R", -1.18 * pulse, 0, 0.12 * pulse)
            rot("foreArm.R", -0.30 * pulse)
            rot("Arm.L", -0.72 * pulse, 0, -0.30 * pulse)
            rot("foreArm.L", -0.32 * pulse)
            rot("Spine", 0.12 * pulse)
            rot("Head", -0.1 * pulse)
            if name == "Chop":
                rot("Arm.R", -1.9 * math.sin(math.pi * t), 0, 0.1)
                rot("Spine2", 0, 0, 0.22 * math.sin(phase))
        elif name == "Climb":
            for s, sign in [("L", 1), ("R", -1)]:
                swing = math.sin(phase) * sign
                rot("Arm." + s, -2.30 + 0.35 * swing, 0, -0.12 * sign)
                rot("foreArm." + s, -0.30 - 0.24 * swing)
                rot("leg." + s, -0.65 - 0.32 * swing)
                rot("knee." + s, 0.90 + 0.32 * swing)
                rot("foot." + s, -0.22)
            rot("Spine", 0.13)
            rot("Head", -0.14)
        elif name == "Fall":
            rot("Arm.L", -0.38, 0, -0.52)
            rot("Arm.R", -0.38, 0, 0.52)
            rot("leg.L", -0.20)
            rot("leg.R", 0.20)
            rot("knee.L", 0.30)
            rot("knee.R", 0.30)
        bpy.context.view_layer.update()
        if name in ("Idle", "Walk", "Cast", "Water", "Fall", "Chop"):
            hand = rig.pose.bones["hand.R"]
            rest = (
                rig.data.bones["hand.R"].matrix_local.inverted()
                @ rig.data.bones["staff.R"].head_local
            )
            grip = hand.matrix @ rest
            direction = (
                Vector((-0.22, 0, 1))
                if name in ("Idle", "Walk", "Fall")
                else Vector((0, -1, -0.08 if name == "Water" else 0.12))
            )
            if name == "Chop":
                strike = 1.7 * math.sin(math.pi * t)
                direction = Vector((0.45, -math.sin(strike), math.cos(strike)))
            orientation = (
                Vector((0, 1, 0))
                .rotation_difference(direction.normalized())
                .to_matrix()
                .to_4x4()
            )
            rig.pose.bones["staff.R"].matrix = Matrix.Translation(grip) @ orientation
            bpy.context.view_layer.update()
        # During climbing the staff is slung on the back, freeing both hands.
        if name == "Climb":
            pb = rig.pose.bones["staff.R"]
            matrix = (
                Matrix.Translation((0.19, 0.23, 0.60))
                @ Matrix.Rotation(-0.32, 4, "Y")
                @ rig.data.bones["staff.R"]
                .matrix_local.to_quaternion()
                .to_matrix()
                .to_4x4()
            )
            pb.matrix = matrix
            bpy.context.view_layer.update()
        if name not in ("Climb", "Fall"):
            evaluated = body.evaluated_get(bpy.context.evaluated_depsgraph_get())
            mesh = evaluated.to_mesh()
            sole = min(
                (evaluated.matrix_world @ mesh.vertices[i].co).z for i in boot_indices
            )
            evaluated.to_mesh_clear()
            # root bone local Y is world Z.
            rig.pose.bones["root"].location.y -= sole
            bpy.context.view_layer.update()
        if name == "Water":
            tip = rig.pose.bones["staff.tip"].head
            grip = rig.pose.bones["staff.R"].head
            axis = (tip - grip).normalized()
            water_keys.append(
                {
                    "time": t,
                    "tip": [-tip.x, tip.z, tip.y],
                    "direction": [-axis.x, axis.z, axis.y],
                }
            )
        for pb in rig.pose.bones:
            for prop in ("rotation_euler", "location", "scale"):
                pb.keyframe_insert(prop, frame=frame, group=pb.name)
        samples.append(
            {
                s: list(rig.pose.bones[s].head)
                for s in ["hand.L", "hand.R", "foot.L", "foot.R", "staff.tip"]
            }
        )
    motion[name] = {
        "frames": count + 1,
        "duration_s": count / 30,
        "hand_travel_m": max(
            (Vector(a["hand.L"]) - Vector(b["hand.L"])).length
            for a in samples
            for b in samples
        ),
    }
    action["loop"] = name in ("Idle", "Walk", "Climb", "Fall")
rig.animation_data.action = actions["Idle"]
bpy.context.scene.frame_set(1)
g.create_studio()
scene = bpy.context.scene
scene.render.resolution_x = 768
scene.render.resolution_y = 768
scene.render.fps = 30
scene.world.node_tree.nodes["Background"].inputs["Color"].default_value = (
    0.10,
    0.15,
    0.19,
    1,
)
scene.world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.4
scene.view_settings.view_transform = "AgX"
# Keep the modeling source inspectable with its studio and all authored actions.
bpy.context.preferences.filepaths.save_version = 0
bpy.ops.wm.save_as_mainfile(filepath=str(OUT / "frieren-chibi.blend"))
g.GLB_PATH = OUT / "frieren-chibi.glb"
g.export_glb()
g.canonicalize_glb()
raw = g.GLB_PATH.read_bytes()
n = struct.unpack_from("<I", raw, 12)[0]
doc = json.loads(raw[20 : 20 + n])
triangles = sum(
    doc["accessors"][p["indices"]]["count"] // 3
    for mesh in doc["meshes"]
    for p in mesh["primitives"]
)
assert {a["name"] for a in doc["animations"]} == set(lengths)
assert triangles <= 12000, triangles
assert motion["Walk"]["hand_travel_m"] > 0.15
assert motion["Climb"]["hand_travel_m"] > 0.12
receipt = {
    "schema": "pocket-openworld.clean-room-character.v1",
    "generator": "generate_chibi.py",
    "source_inputs": [
        "generate_chibi.py",
        "generate_character.py (repository-owned primitive helpers only)",
    ],
    "external_assets_read": [],
    "blender": bpy.app.version_string,
    "triangles": triangles,
    "materials": len(doc["materials"]),
    "primitives": sum(len(m["primitives"]) for m in doc["meshes"]),
    "bones": len(bones),
    "body_height_m": 1.78,
    "head_height_m": 0.70,
    "head_count": 1.78 / 0.70,
    "clips": motion,
    "sockets": ["hand.L", "staff.R", "staff.tip"],
    "sha256": hashlib.sha256(raw).hexdigest(),
}
(OUT / "chibi-water-sockets.json").write_text(json.dumps(water_keys, indent=2) + "\n")
(OUT / "frieren-chibi-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
if not args.skip_previews:
    previews = OUT / "chibi-previews"
    previews.mkdir(exist_ok=True)
    for label, clip, frame, camera in [
        ("idle-front", "Idle", 1, (2.6, -5.8, 2.7)),
        ("idle-rear", "Idle", 1, (-2.7, 5.8, 2.6)),
        ("walk", "Walk", 6, (2.6, -5.8, 2.4)),
        ("walk-side", "Walk", 10, (5, -0.4, 2.0)),
        ("chop", "Chop", 14, (2.7, -5.8, 2.6)),
        ("cast", "Cast", 16, (2.7, -5.8, 2.6)),
        ("water", "Water", 16, (2.7, -5.8, 2.6)),
        ("climb", "Climb", 10, (3.4, -5.5, 2.5)),
        ("fall", "Fall", 12, (2.6, -5.8, 2.5)),
    ]:
        rig.animation_data.action = actions[clip]
        scene.frame_set(frame)
        scene.camera.location = camera
        scene.camera.data.type = "ORTHO"
        scene.camera.data.ortho_scale = 2.25
        g.look_at(scene.camera, Vector((0, 0, 0.89)))
        scene.render.filepath = str(previews / (label + ".png"))
        bpy.ops.render.render(write_still=True)
print(json.dumps(receipt, indent=2))
