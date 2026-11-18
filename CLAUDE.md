# Retreat-CoreSDK

A shared, reusable toolbox of common utilities, mathematical functions, data structures, helper classes, and foundational abstractions used by the other Retreat projects (the engine and the gameplay modules).

This file contains project facts and rules only. Personal preferences belong in `CLAUDE.local.md`.

The root `../CLAUDE.md` also applies here. This file may narrow its scope (see "Status") but not relax its rules; its verification and infrastructure rules apply only once this repository has code that needs them.

## Status

Early stage. First crate: `retreat_rng`. Do not implement anything beyond what is explicitly requested.

## Layout

Cargo workspace (`Cargo.toml`); one small crate per area under `crates/`:

- `crates/retreat_rng/` — `RetreatRng`, seeded deterministic random numbers (SplitMix64). Consumers: the engine's
  command-script runner and gameplay modules.

## Decisions

- **`retreat_rng` implements its algorithm itself (SplitMix64) and has no dependencies.** A seed must produce the same
  sequence on every platform and in every version (scripts, replays and native-vs-WASM checks depend on it); with our
  own algorithm, no dependency update can change it. Changing an existing seed's output is a breaking change; the
  reference-vector test guards it. Not for security (predictable by design). Revisit if a better-quality generator
  is needed (e.g. xoshiro256**), as a new type, not by changing `RetreatRng`.

## What belongs here

CoreSDK holds generic building blocks that are shared across modules and the engine, and that carry no game meaning: math, geometry, random numbers, timers, collections, curves, error types, and similar.

**Test:** would this make sense in a non-game Rust program? If yes, it can belong here. If it only makes sense inside a game, it does not.

**Does not belong here:**
- Shared gameplay concepts (an item, a damage event, health). They define how modules cooperate and live in the engine's public API / contract crates.
- Code that depends on engine types. It is not generic.
- Code used by only one module or only the engine. It stays where it is used.

## How things get added

- **Promote on second use.** A helper starts inside the first module (or the engine) that needs it. When a second consumer needs it, move it here. Do not add code speculatively.
- Everything shared across modules that is generic goes here; never duplicate it between modules and never put it in the engine.

## Constraints

- Depends on nothing else in Retreat. Dependency direction is: CoreSDK, then contract crates, then engine and modules, then a game. Never reverse it.
- Must build for both native and `wasm32-unknown-unknown`. No threads, filesystem, or sockets unless gated and not required by consumers.
- Split by area into small crates, so a consumer only pulls in what it uses.
- This is the repository most sensitive to change: a change reaches every module and the engine. Keep the public surface small, test it thoroughly, and version it carefully. Breaking changes need a deliberate decision.

## Working rules

- Keep the initial implementation as simple as reasonably possible.
- Do not modify sibling repositories from this one without being asked.
- Write documentation and code comments in English.
