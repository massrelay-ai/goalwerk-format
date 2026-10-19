# Contributing

This repository is the open format of Atingo and its tools: the pack schema, the validator, the `atingo-path` crate and the typed client.

## Sign-off (DCO)

Every commit carries a `Signed-off-by: Your Name <you@example.org>` line (`git commit -s`). It states that you wrote the change or have the right to submit it
under the Apache License 2.0 (the Developer Certificate of Origin, https://developercertificate.org).

## The validator is the gate

A change is accepted when these pass, from a clean checkout:

```sh
npm ci
npm run packs:check      # the sample pack, zero warnings
npm run rust:check       # the crate compiles
npm run client:check     # the typed client is current and type-checks
```

A change to the pack schema updates `packs/atingo/SCHEMA.md`, the JSON schemas and the Rust crate together; the two validators must report the same codes.

## Writing a path

A path is data: JSON files in the folders `examples/sample-pack/` shows. Run `node tools/packs/check.mjs <your pack folder>` until it prints `0 warnings`. A path you
contribute to the hosted product enters as a `creator` path under your own licence (CC BY-SA 4.0 by default; all rights reserved is allowed). The platform tier,
its curricula, personas and schedules are the project's own and are not accepted as pull requests.

## What a path may not do

No step is closed by a say-so; only a receipt the kernel verified closes one. No personal ground (age, gender, health, faith, nationality, orientation) is ever a key,
a filter or a segment. The validator refuses both.
