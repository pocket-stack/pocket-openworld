//! Compile the PocketJS application into this Cargo build's private output.
use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=POCKET_OPENWORLD_BUN");
    println!("cargo:rerun-if-env-changed=PATH");
    for path in ["build.rs", ".bun-version", "tools/build_ui.ts", "ui"] {
        println!("cargo:rerun-if-changed={path}");
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("ui");
    let bun = env::var_os("POCKET_OPENWORLD_BUN").unwrap_or_else(|| "bun".into());
    let status = Command::new(&bun)
        .current_dir(&root)
        .arg("tools/build_ui.ts")
        .arg(format!("--outdir={}", output.display()))
        .status()
        .unwrap_or_else(|error| {
            panic!("cannot run {bun:?}: {error}; install Bun {} (see .bun-version), then run Cargo again", fs::read_to_string(root.join(".bun-version")).unwrap().trim())
        });
    assert!(
        status.success(),
        "PocketJS UI generation failed; Cargo will not embed an older bundle"
    );

    // Use the compiler's dependency graph, including transitive UI/compiler
    // modules, font files, the lockfile, installed packages and the Bun binary.
    let inputs = fs::read_to_string(output.join("cargo-inputs.txt")).unwrap();
    for path in inputs.lines() {
        println!("cargo:rerun-if-changed={path}");
    }
    // Deleting an output must trigger generation even if sources did not change.
    for name in [
        "main.js",
        "main.pak",
        "manifest.json",
        "cargo-inputs.txt",
        "cache.json",
    ] {
        let path = output.join(name);
        assert!(
            fs::metadata(&path).is_ok_and(|m| m.len() > 0),
            "missing generated UI output: {}",
            path.display()
        );
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
