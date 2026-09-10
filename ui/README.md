# Embedded PocketJS controls and HUD

These TSX modules are the source of the control window and HUD. Cargo's
`build.rs` invokes `tools/build_ui.ts` and the pinned PocketJS compiler to
generate `main.js`, `main.pak` and `manifest.json` under `OUT_DIR/ui`.
Only source and licenses belong here; generated files are not committed.

Rebuild from the repository root:

```sh
cargo build --locked
```

The window uses the shared `XP_THEME` and overlay input channel from PocketJS.
Every gameplay HUD label uses the same PocketJS text components, Inter font
slots and display scale. Paragraph wrapping and notification sizing use the
native font measurement provider. The HUD stays passive; opening the controls
is what returns pointer ownership to the UI.
The geometric chrome is authored code. The pak includes font atlases baked from
Inter Regular and Bold, copyright 2016 The Inter Project Authors, distributed
under the SIL Open Font License 1.1 in [LICENSE-Inter.txt](LICENSE-Inter.txt).
The generated JS includes PocketJS and SolidJS code under their MIT licenses.
See [PocketJS](../vendor/pocketjs/LICENSE) and [SolidJS](LICENSE-SolidJS.txt).
