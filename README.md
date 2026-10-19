# Atingo format

Atingo is goal-based learning with executed, verified steps. This repository is the open part of it: the **format** a learning path is written in, and the **tools** that
validate it. Anyone can write a path, check it offline, and let another app or an n8n flow read it.

## What is here

| Path | What it is |
|---|---|
| `packs/atingo/SCHEMA.md` | the pack schema: paths, nodes, goal kinds, lesson templates, follow-up, evidence, personas |
| `packs/atingo/goal-kinds/` | the goal-kind format, with five rows. A goal kind is a schema row, not a curriculum |
| `tools/packs/` | the validator (`check.mjs`), its JSON schemas, the kernel pins |
| `crates/atingo-path/` | the same validation as a Rust crate (with a WASM build) and the `atingo-path` command |
| `packages/client/` | `@atingo/client`, the typed client for the kernel's public doors |
| `examples/sample-pack/` | a small synthetic pack that passes the validator with no warnings |

## Validate a path

```sh
npm ci
npm run packs:check                              # the sample pack
node tools/packs/check.mjs path/to/your/pack     # your own pack folder
npm run rust:check                               # the Rust crate compiles
npm run client:check                             # the typed client is current and type-checks
```

A pack is data, never code. The validator refuses a path whose nodes have no sources when it is priced, a persona that instructs a forbidden act, a graph with a
prerequisite cycle, and a motive keyed by a protected ground.

## What is not here

The curricula, the personas and schedules of the hosted product, its brand and its app are not part of this repository. A path you write enters the product as a
`creator` path, under your own licence.

## Licences

| Part | Licence |
|---|---|
| code, schemas, tools, the crate and the client | Apache-2.0 (`LICENSE`) |
| `examples/sample-pack/` content | CC BY-SA 4.0 |

Contributions: see `CONTRIBUTING.md` (DCO sign-off; the validator is the gate).
