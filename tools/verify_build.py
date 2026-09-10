#!/usr/bin/env python3
"""Exercise production UI build hooks in a disposable, dependency-free Rust host.

Copies source inputs only. All mutations happen in the temporary fixture, never
in the application checkout or its pinned engine. The regular CI build/tests
exercise the full game; this fixture isolates generation and Cargo invalidation.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from verify_shelter import read_ui_manifest


def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/pocket-openworld"))
    parser.add_argument("--cargo", default=os.environ.get("CARGO", "cargo"))
    args = parser.parse_args()
    root = Path.cwd()
    engine = root / "vendor/pocketjs"
    manifest = read_ui_manifest(args.binary)
    with tempfile.TemporaryDirectory(prefix="pocket-openworld-build-") as directory:
        work = Path(directory)
        for name in manifest["sources"]:
            copy(root / name, work / name)
        for name in manifest["engineSources"]:
            copy(engine / name, work / "vendor/pocketjs" / name)
        copy(root / "rust-toolchain.toml", work / "rust-toolchain.toml")
        (work / "Cargo.toml").write_text('[package]\nname = "ui-build-contract"\nversion = "0.0.0"\nedition = "2024"\n')
        (work / "Cargo.lock").write_text('version = 4\n\n[[package]]\nname = "ui-build-contract"\nversion = "0.0.0"\n')
        (work / "src").mkdir()
        (work / "src/main.rs").write_text('''
const _: &str = include_str!(concat!(env!("OUT_DIR"), "/ui/main.js"));
const _: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ui/main.pak"));
fn main() { print!("{}", include_str!(concat!(env!("OUT_DIR"), "/ui/manifest.json"))); }
''')
        environment = {**os.environ, "CARGO_TARGET_DIR": str(work / "target")}
        last_build_log = ""

        def build(fail=False, env=None):
            nonlocal last_build_log
            command = [args.cargo, "build", "--locked", "--offline", "--message-format=json"]
            result = subprocess.run(command, cwd=work, env=env or environment, capture_output=True, text=True)
            last_build_log = result.stderr
            if fail:
                assert result.returncode != 0, "generation failure incorrectly reused an older bundle"
                return result.stdout + result.stderr
            if result.returncode:
                raise RuntimeError(result.stdout + result.stderr)
            messages = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
            output = next(Path(m["out_dir"]) / "ui" for m in messages if m["reason"] == "build-script-executed")
            executable = next(Path(m["executable"]) for m in messages if m["reason"] == "compiler-artifact" and m.get("executable"))
            return executable, output

        def info(executable):
            return read_ui_manifest(executable, work, work / "vendor/pocketjs")

        assert not (work / "node_modules").exists()
        assert not (work / "vendor/pocketjs/node_modules").exists()
        executable, output = build()
        original = info(executable)
        assert not (work / "node_modules").exists(), "application symlinks must not be required"
        assert not (work / "assets/ui").exists()
        print("PASS first Cargo build generates UI and installs locked dependencies", flush=True)

        timestamp = (output / "manifest.json").stat().st_mtime_ns
        build()
        assert (output / "manifest.json").stat().st_mtime_ns == timestamp, "unchanged sources rebuilt UI:\n" + last_build_log
        print("PASS unchanged Cargo build reuses generated UI", flush=True)

        old_binary = work / executable.name
        copy(executable, old_binary)
        ui = work / "ui/main.tsx"
        original_ui = ui.read_text()
        assert "Pocket Openworld" in original_ui
        ui.write_text(original_ui.replace("Pocket Openworld", "Pocket Build Check"))
        try:
            info(old_binary)
        except AssertionError as error:
            assert "stale embedded UI" in str(error)
        else:
            raise AssertionError("acceptance accepted an executable from older UI sources")
        build()
        changed = info(executable)
        assert changed["artifacts"]["main.js"] != original["artifacts"]["main.js"]
        print("PASS UI source edit rebuilds the embed; old binary is rejected", flush=True)

        theme = work / "vendor/pocketjs/framework/src/themes/desktop.ts"
        original_theme = theme.read_text()
        assert "#174bb8" in original_theme
        theme.write_text(original_theme.replace("#174bb8", "#193d85"))
        build()
        themed = info(executable)
        assert themed["artifacts"]["main.pak"] != changed["artifacts"]["main.pak"]
        print("PASS shared theme edit rebuilds baked styles", flush=True)

        font = work / "vendor/pocketjs/assets/fonts/Inter-Regular.ttf"
        original_font = font.read_bytes()
        font.write_bytes((engine / "assets/fonts/InterDisplay-Regular.ttf").read_bytes())
        build()
        assert info(executable)["artifacts"]["main.pak"] != themed["artifacts"]["main.pak"]
        print("PASS font edit rebuilds glyph atlases", flush=True)

        ui.write_text(original_ui)
        theme.write_text(original_theme)
        font.write_bytes(original_font)
        build()
        assert info(executable)["artifacts"] == original["artifacts"], "restored sources produced different UI artifacts"
        (output / "main.pak").unlink()
        build()
        assert info(executable)["artifacts"] == original["artifacts"]
        assert (output / "main.pak").is_file()
        print("PASS reproducible rebuild and missing-output recovery", flush=True)

        error = build(fail=True, env={**environment, "POCKET_OPENWORLD_BUN": str(work / "missing-bun")})
        assert "cannot run" in error and "install Bun" in error
        build()
        ui.write_text(original_ui + "\nexport const broken = ;\n")
        error = build(fail=True)
        assert "UI generation failed" in error
        ui.write_text(original_ui)
        build()
        assert info(executable)["artifacts"] == original["artifacts"]
        print("PASS missing tool/compiler errors fail the build and recover", flush=True)

        # Execute the real game from an empty directory with no Bun in PATH.
        standalone = work / "standalone"
        standalone.mkdir()
        game = standalone / args.binary.name
        copy(args.binary, game)
        result = subprocess.run([str(game), "--headless", "--scenario", "idle", "--ticks", "1",
                                 "--receipt", str(standalone / "idle.json")], cwd=standalone,
                                env={**os.environ, "PATH": "/usr/bin:/bin"}, capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        print("PASS built game runs without Bun, source files or loose UI assets", flush=True)


if __name__ == "__main__":
    main()
