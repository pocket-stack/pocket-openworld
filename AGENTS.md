# Repository instructions

- Keep Pocket Openworld application code, recipes, controls, procedural art,
  character assets, and scenario receipts in this repository. Put reusable
  Pocket3D rendering, asset, animation, or simulation mechanisms in PocketJS
  and advance the pinned `vendor/pocketjs` gitlink only after those changes
  land there.
- Collision, integration, attachment, structure, and reaction solvers must not
  branch on an entity ID, tag, recipe, or scenario. Change a shared physical
  rule, state its invariant, and test it across at least two entity, material,
  or collider configurations. Scenario regressions are additional coverage.
- Treat state receipts as simulation evidence and screenshots as rendering
  evidence. A gameplay acceptance change that affects both must verify both.
- Use Conventional Commits for commits and pull request titles. Open completed
  changes as Draft pull requests, then mark them ready only after the relevant
  tests and headless scenarios pass.
