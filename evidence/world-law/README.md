# Frozen World Law experiment evidence

This directory freezes the final local acceptance snapshot for the EVA/Titan
World Law experiment. It is branch evidence, not a release artifact.

Generation baseline:

- Date: 2026-08-23 (Asia/Shanghai)
- Pocket Openworld source: `2e27026cb569af2e6b2e74905da3cdcd9c41ffc3`
- PocketJS runtime source: `0f38db65b740c3a5678107fd8d55375f90e1a2d8`
- Seed: `7`
- Render size: `1440x900`
- GPU used for the render capture: Apple M3 Max

| Scenario | Ticks | Visible state hash | Hidden state hash | Acceptance |
| --- | ---: | --- | --- | --- |
| `eva-at-field` | 190 | `00c703a156f42df9` | `61916dc7000e79e5` | `true` |
| `titan-paths` | 300 | `05290374e9391ada` | `3979bd8f1a328027` | `true` |
| `world-law-crossover` | 190 | `6be53798f4950106` | `6ee66afb0a5f6fc5` | `true` |

Each JSON file is simulation evidence from the deterministic state receipt.
The matching PNG is rendering evidence from the same capture turn. Both are
kept because neither one substitutes for the other.

Regenerate from the repository root with:

```sh
cargo run --locked --package pocket-openworld -- \
  --headless --scenario eva-at-field --ticks 190 --seed 7 --size 1440x900 \
  --receipt evidence/world-law/eva-at-field.json \
  --screenshot evidence/world-law/eva-at-field.png

cargo run --locked --package pocket-openworld -- \
  --headless --scenario titan-paths --ticks 300 --seed 7 --size 1440x900 \
  --receipt evidence/world-law/titan-paths.json \
  --screenshot evidence/world-law/titan-paths.png

cargo run --locked --package pocket-openworld -- \
  --headless --scenario world-law-crossover --ticks 190 --seed 7 --size 1440x900 \
  --receipt evidence/world-law/world-law-crossover.json \
  --screenshot evidence/world-law/world-law-crossover.png
```

`SHA256SUMS` fixes the exact checked-in evidence bytes.
