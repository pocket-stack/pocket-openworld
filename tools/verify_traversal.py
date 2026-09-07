#!/usr/bin/env python3
"""Run live-input simulation + GPU scenarios and exact receipt replay."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
SCENARIOS = {
    "idle": 1,
    "character-walk": 45,
    "character-chop": 119,
    "character-cast": 24,
    "character-water": 47,
    "character-carry": 360,
    "orchard-fire": 720,
    "grass-fire": 48,
    "grass-burnout": 210,
    "campfire-douse": 390,
    "tree-climb": 150,
    "tree-traverse": 210,
    "climb-release": 175,
    "climb-jump": 165,
    "slope-climb": 260,
    "slope-walk": 130,
    "slope-crest": 500,
}


def write_receipt(path, receipt):
    """Keep full state while storing each entity on one reviewable JSON line."""
    header = json.dumps({k: v for k, v in receipt.items() if k != "entities"}, indent=2)
    entities = ",\n".join(
        "    " + json.dumps(e, separators=(",", ":")) for e in receipt["entities"]
    )
    path.write_text(
        header.removesuffix("\n}") + ',\n  "entities": [\n' + entities + "\n  ]\n}\n"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--binary", type=Path, default=ROOT / "target/debug/pocket-openworld"
    )
    parser.add_argument("--output", type=Path, default=ROOT / "evidence/chibi-climbing")
    parser.add_argument(
        "--engine-checkout", type=Path, default=ROOT / "vendor/pocketjs"
    )
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    character_hash = hashlib.sha256(
        (ROOT / "assets/character/frieren-chibi.glb").read_bytes()
    ).hexdigest()
    generated = json.loads(
        (ROOT / "assets/character/frieren-chibi-receipt.json").read_text()
    )
    imported = json.loads(
        (ROOT / "assets/character/frieren-chibi-import-qa.json").read_text()
    )
    assert generated["sha256"] == imported["glb_sha256"] == character_hash, (
        "stale character QA receipts"
    )
    summary = {}
    for name, ticks in SCENARIOS.items():
        path = args.output / name
        command = [
            str(args.binary.resolve()),
            "--headless",
            "--scenario",
            name,
            "--ticks",
            str(ticks),
            "--seed",
            "7",
            "--size",
            "1200x750",
        ]
        subprocess.run(
            command
            + [
                "--receipt",
                str(path.with_suffix(".json")),
                "--screenshot",
                str(path.with_suffix(".png")),
            ],
            check=True,
            cwd=ROOT,
        )
        receipt = json.loads(path.with_suffix(".json").read_text())
        write_receipt(path.with_suffix(".json"), receipt)
        with tempfile.TemporaryDirectory() as temp:
            replay = Path(temp) / "replay.json"
            subprocess.run(
                command + ["--receipt", str(replay)],
                check=True,
                cwd=ROOT,
                stdout=subprocess.DEVNULL,
            )
            assert receipt == json.loads(replay.read_text()), (
                f"{name} receipt replay diverged"
            )
        motion = receipt["locomotion"]
        mode = motion["current"]["mode"]
        if name in ("tree-climb", "tree-traverse", "slope-climb"):
            assert (
                mode == "Climbing"
                and motion["climbing_turns"] > 80
                and motion["max_height"] > 2.0
            )
        if name == "tree-traverse":
            assert motion["surface_normal"][0] > 0.9
        if name == "climb-release":
            assert mode == "Airborne" and motion["airborne_turns"] > 20
        if name == "climb-jump":
            assert mode == "Airborne" and motion["airborne_turns"] > 10
            player = next(e for e in receipt["entities"] if "player" in e["tags"])
            assert player["position"][2] > 1.1
        if name == "slope-walk":
            assert motion["sliding_turns"] > 20 and motion["climbing_turns"] == 0
        if name == "slope-crest":
            assert (
                mode == "Grounded"
                and motion["climbing_turns"] > 80
                and motion["max_height"] > 4.5
            )
        summary[name] = {
            "ticks": ticks,
            "hash": receipt["state_hash"],
            "mode": mode,
            "receipt_replayed": True,
            "png_sha256": hashlib.sha256(
                path.with_suffix(".png").read_bytes()
            ).hexdigest(),
        }
        print(f"PASS {name}: {mode}, hash {receipt['state_hash']}", flush=True)
    summary = {
        "engine_revision": subprocess.check_output(
            ["git", "-C", str(args.engine_checkout), "rev-parse", "HEAD"], text=True
        ).strip(),
        "character_sha256": hashlib.sha256(
            (ROOT / "assets/character/frieren-chibi.glb").read_bytes()
        ).hexdigest(),
        "binary_sha256": hashlib.sha256(args.binary.resolve().read_bytes()).hexdigest(),
        "scenarios": summary,
    }
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")


if __name__ == "__main__":
    main()
