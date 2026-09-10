# Build and acceptance output

Install the Rust toolchain in `rust-toolchain.toml` and Bun version in
`.bun-version`, initialize submodules, then run `cargo run --locked`.
Cargo invokes the UI compiler on the first build and when its inputs change.
`POCKET_OPENWORLD_BUN` can select the Bun executable; its version is checked.

`tools/build_ui.ts` installs dependencies with the vendored frozen Bun lockfile
and builds the TSX entry through PocketJS. The compiler supplies the dependency
graph, including transitive application/framework modules, fonts and packages.
Cargo watches those files, the UI source directory and its generated outputs.
An unchanged build reuses its outputs. Deleting an output regenerates it.
The generator validates cached input and output hashes before reuse, including
the UI resource tree, so Cargo's output-file checks do not recompile unchanged UI.
Missing Bun or compiler failures stop the build instead of embedding old files.

JS, PAK, dependency records and the build manifest are written under Cargo's
`OUT_DIR/ui`; no source-tree bundle is loaded as a fallback. The executable
embeds JS/PAK and their build provenance and runs without Bun or source files.
`--ui-build-info` prints that embedded provenance without initializing a GPU.

## Verification

```sh
cargo build --locked
python3 tools/verify_build.py
cargo test --locked
python3 tools/verify_shelter.py --output target/acceptance/controls
```

`verify_build.py` creates a temporary Rust host with the production build hook
and copies only the application/compiler input sources. It starts without
generated output or installed JavaScript dependencies. It verifies first build,
no-op rebuild, application/theme/font edits, rejection of an old executable,
missing-output recovery and failure/recovery after missing Bun or invalid TSX.
It also launches a copy of the real game from an empty directory without Bun
in PATH. It does not modify the application checkout or its engine dependency.

`verify_shelter.py` reads the manifest from the executable being tested and
checks source hashes before running scenarios. It records full state receipts,
compares complete deterministic replays and validates the GPU screenshots.
These are per-run outputs, not golden files used by a regression comparison.

## Repository and CI

Keep UI source, font/software licenses, build scripts, scenario definitions,
assertions and manual acceptance instructions in Git. Generated UI bundles and
full receipts/screenshots/logs are ignored. `tools/check_generated.py` checks
the Git index in CI and rejects output under the old `assets/ui` / `evidence`
locations and build/artifact directories.

The CI workflow builds from source and uploads `pocket-openworld-acceptance`
with receipts, PNGs, logs, build provenance and the executable hash. Artifacts
are retained for 30 days. PR evidence links point to the run for its current
commit; rerun the workflow after expiry. A repository screenshot gallery is not
required to review or reproduce acceptance.
