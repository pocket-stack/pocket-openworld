#!/usr/bin/env python3
"""Exercise live controls, compare complete replay receipts, and check PNG files."""

import argparse
import binascii
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import zlib


SCENARIOS = [
    ("controls-open", 4),
    ("controls-rain", 906),
    ("controls-screen", 1206),
    ("controls-screen-water", 2496),
    ("controls-firebreak", 1806),
    ("controls-firebreak-dry", 1806),
    ("controls-drag", 8),
    ("controls-return", 12),
    ("shelter-rain", 480),
    ("shelter-rain-moved", 900),
    ("shelter-screen", 1200),
    ("shelter-screen-water", 1290),
    ("shelter-dry", 2400),
    ("shelter-screen-open-water", 2490),
    ("shelter-firebreak", 1800),
    ("shelter-firebreak-dry", 1800),
    ("idle", 1),
    ("character-walk", 45),
    ("character-chop", 119),
    ("character-cast", 24),
    ("character-water", 47),
    ("character-carry", 360),
    ("orchard-fire", 720),
    ("grass-fire", 48),
    ("grass-burnout", 210),
    ("campfire-douse", 390),
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_ui_manifest(binary, root=Path("."), engine=Path("vendor/pocketjs")):
    """Validate the UI identity embedded in this executable, not a loose sidecar."""
    run = subprocess.run([str(binary.resolve()), "--ui-build-info"], capture_output=True, text=True)
    if run.returncode:
        raise RuntimeError(f"cannot read embedded UI build info; run cargo build --locked:\n{run.stderr}")
    manifest = json.loads(run.stdout)
    assert manifest["schema"] == 1, "unsupported UI build manifest"
    assert manifest["bunVersion"] == (root / ".bun-version").read_text().strip()
    for group, base in [("sources", root), ("engineSources", engine)]:
        assert manifest[group], f"missing UI build inputs: {group}"
        for name, expected in manifest[group].items():
            assert digest(base / name) == expected, f"stale embedded UI: {base / name}; run cargo build --locked"
    return manifest


def check_png(path, expected_size):
    """Validate every chunk and the complete decompressed scanline stream."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    offset = 8
    compressed = bytearray()
    ended = False
    while offset < len(data):
        length = struct.unpack_from(">I", data, offset)[0]
        kind = data[offset + 4 : offset + 8]
        body = data[offset + 8 : offset + 8 + length]
        crc = struct.unpack_from(">I", data, offset + 8 + length)[0]
        assert binascii.crc32(kind + body) & 0xFFFFFFFF == crc, path
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(
                ">IIBBBBB", body
            )
            assert (width, height) == expected_size, path
            assert depth == 8 and color in (2, 6), path
            assert (compression, filtering, interlace) == (0, 0, 0), path
        elif kind == b"IDAT":
            compressed.extend(body)
        elif kind == b"IEND":
            ended = True
        offset += length + 12
    assert ended and offset == len(data), path
    decoder = zlib.decompressobj()
    pixels = decoder.decompress(compressed) + decoder.flush()
    assert decoder.eof and not decoder.unused_data, path
    stride = width * (4 if color == 6 else 3) + 1
    assert len(pixels) == stride * height, path
    assert all(pixels[row * stride] <= 4 for row in range(height)), path


def compact_receipt(data):
    """Keep complete entity state with one entity per line for reviewable diffs."""
    lines = []
    for key, value in data.items():
        if key == "entities":
            body = ",\n".join("    " + json.dumps(entity) for entity in value)
            lines.append(f'  "entities": [\n{body}\n  ]')
        else:
            encoded = json.dumps(value, indent=2).replace("\n", "\n  ")
            lines.append(f"  {json.dumps(key)}: {encoded}")
    return "{\n" + ",\n".join(lines) + "\n}\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/pocket-openworld"))
    parser.add_argument("--output", type=Path, default=Path("target/acceptance/shelter"))
    parser.add_argument("--engine-checkout", type=Path, default=Path("vendor/pocketjs"))
    parser.add_argument("--size", default="960x600")
    parser.add_argument("--ui-scale", type=float, default=1.0)
    parser.add_argument("--scenario", action="append", choices=[name for name, _ in SCENARIOS])
    args = parser.parse_args()
    ui_manifest = read_ui_manifest(args.binary, engine=args.engine_checkout)
    args.output.mkdir(parents=True, exist_ok=True)
    expected_size = tuple(map(int, args.size.split("x")))
    binary = args.binary.resolve()
    results = []
    for scenario, ticks in SCENARIOS:
        if args.scenario and scenario not in args.scenario:
            continue
        receipt = args.output / f"{scenario}.json"
        screenshot = args.output / f"{scenario}.png"
        base = [str(binary), "--headless", "--scenario", scenario, "--ticks", str(ticks),
                "--seed", "7", "--size", args.size, "--ui-scale", str(args.ui_scale)]
        run = subprocess.run(base + ["--receipt", str(receipt), "--screenshot", str(screenshot)],
                             capture_output=True, text=True)
        (args.output / f"{scenario}.log").write_text(run.stdout + run.stderr)
        if run.returncode:
            raise RuntimeError(f"{scenario}:\n{run.stdout}\n{run.stderr}")
        first = json.loads(receipt.read_text())
        replay = args.output / f"{scenario}.replay.json"
        rerun = subprocess.run(base + ["--receipt", str(replay)], capture_output=True, text=True)
        if rerun.returncode:
            raise RuntimeError(f"{scenario} replay:\n{rerun.stdout}\n{rerun.stderr}")
        assert first == json.loads(replay.read_text()), f"{scenario}: replay differs"
        replay.unlink()
        check_png(screenshot, expected_size)
        receipt.write_text(compact_receipt(first))
        results.append({"scenario": scenario, "ticks": ticks, "state_hash": first["state_hash"],
                        "replay_identical": True, "png_sha256": digest(screenshot),
                        "receipt_sha256": digest(receipt)})
        print(f"PASS {scenario}: live inputs, acceptance, replay, PNG", flush=True)
    source_paths = sorted(Path("src").glob("*.rs")) + [Path("Cargo.toml"), Path("Cargo.lock"),
                   Path("assets/character/frieren.glb"), Path("tools/verify_shelter.py"), *sorted(Path("ui").glob("*.tsx")),
                   Path("tools/build_ui.ts"), Path("build.rs"), Path(".bun-version")]
    summary = {"size": args.size, "ui_scale": args.ui_scale, "binary_sha256": digest(binary),
               "pocketjs_revision": subprocess.check_output(
                   ["git", "-C", str(args.engine_checkout), "rev-parse", "HEAD"], text=True).strip(),
               "sources": {str(path): digest(path) for path in source_paths}, "ui_build": ui_manifest, "scenarios": results}
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"All {len(results)} scenarios passed: {args.output}")


if __name__ == "__main__":
    main()
