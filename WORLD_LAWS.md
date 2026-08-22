# World Law Lab design

## Goal and boundary

The POC proves that a fictional ontology can become authoritative world state
without turning a character into a bag of special-case abilities:

```text
hidden state -> persistent relation -> law -> projection -> visible physics
```

`pocket3d-world::WorldLawRuntime` owns the reusable renderer-free primitives:

- typed hidden values attached to stable entity IDs;
- persistent, weighted relations;
- finite spatial fields with local opposing-field interference;
- deferred state, relation, field, transition, projection, and
  materialization commands;
- one deterministic transition per entity per transaction;
- snapshot, restore, ordered events, and a hidden-state hash.

This repository owns the Evangelion- and Titan-themed law packs,
constitutions, recipes, scripted experiments, attributed character-model
adaptations, and acceptance receipts. No character, scenario, or crossover
name appears in the generic runtime or physical solver.

## Fixed-turn execution

Every application tick runs the same sequence:

1. Observable conditions such as injury and intent queue hidden-state changes.
2. Law packs read the previous committed hidden state and relations.
3. They derive or update spatial fields and discrete transitions.
4. Every dynamic visible body is probed against every relevant field without
   inspecting tags, recipes, or character IDs.
5. Projections queue ordinary physical impulses; materialization spawns
   ordinary bodies and attachments.
6. The visible world advances through its existing fixed physics/reaction
   step, then fracture events feed the next hidden-world transaction.

This one-turn transaction boundary makes the system replayable and prevents a
physics callback from mutating hidden state midway through a solver phase.

## Shared field invariant

For the spherical barrier projection, only inward normal motion is changed.
If a body has inward normal kinetic energy `K` and the locally resolved field
budget is `F`, the projection removes exactly `min(K, F)`:

```text
0 <= energy removed <= min(incoming normal energy, field budget)
tangential velocity is unchanged
resulting kinetic energy never exceeds incoming kinetic energy
```

Opposing fields cancel only at points contained by both field volumes. A
second field therefore weakens a local boundary without switching off the
source everywhere. The runtime tests this invariant independently, and the
application exercises the same projection with sphere and capsule bodies.

## EVA law pack

The EVA constitution is data composed from three entities:

```text
Pilot --synchronized-with--> Soul --embodied-by--> Unit
                                  |
                             identity state
                                  |
                       identity-boundary field
                                  |
                         finite physical impulse
```

The `eva-at-field` chamber launches ordinary matter into the boundary. The
first projectile is stopped. A second, opposing identity field then overlaps
one region of the boundary; an otherwise identical projectile loses only the
remaining local field budget and penetrates. The receipt records both the
hidden relation/field state and the visible projectile results.

## Titan law pack

The host constitution declares Subject, Titan Power, and a persistent Paths
connection. Materialization is legal only when all four independent
conditions are present:

```text
injury + intent + titan power + Paths connection
                         |
                 titan materialization
                         |
        torso/head/arms/legs become physical entities
```

The `titan-paths` chamber builds the avatar through staged bone, muscle, and
skin projections. Its ordinary right-arm structure is then cut by the normal
structure solver. The fractured arm detaches and falls under normal gravity.
The morphology law observes the missing slot, uses the existing Paths
relation, and materializes a new attached physical arm. The old arm remains a
separate fallen body, so regeneration is not a health-bar reset.

The visible body and both old and regenerated arms come from the same
attributed source model. The arm is baked as a shoulder-rooted GLB, so its
render transform follows the ordinary attached/detached entity throughout the
transaction; the source model never changes the solver contract.

## Cross-law chamber

`world-law-crossover` sends two capsule-shaped biomass impacts into the exact
same identity-boundary projection used by the EVA chamber:

- a body whose incoming normal energy is below the field budget stops;
- a body whose energy exceeds the budget is attenuated but penetrates.

The implementation has no `TitanVsEvaRule`, entity-name branch, or tag check.
The receipt publishes `crossover_specific_rule_count: 0` alongside the two
energy budgets and visible outcomes.

## Evidence

Each chamber has a fixed capture turn so the JSON state receipt and PNG render
describe the same moment:

```sh
cargo run --locked -- --headless --scenario eva-at-field --ticks 190 \
  --receipt /tmp/eva-at-field.json --screenshot /tmp/eva-at-field.png

cargo run --locked -- --headless --scenario titan-paths --ticks 300 \
  --receipt /tmp/titan-paths.json --screenshot /tmp/titan-paths.png

cargo run --locked -- --headless --scenario world-law-crossover --ticks 190 \
  --receipt /tmp/world-law-crossover.json \
  --screenshot /tmp/world-law-crossover.png
```

The JSON proves hidden and visible simulation state; the PNG proves only the
projection's presentation. Both are required for gameplay acceptance.
