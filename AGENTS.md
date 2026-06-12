# AGENTS.md

## Project overview

Deviruchi is a Ragnarok Online game server rewritten in Rust, based on rAthena. Single binary contains Login (6900), Char (6000), and Map (6121) servers. Also ships `devi-agent`, an AI ops assistant. 83k+ lines, 1389 tests.

## Build & test commands

```bash
cargo build --release                    # server binary
cargo build --release -p devi-agent      # AI assistant binary
cargo test                               # all tests (unit + integration, ~1389)
cargo test <module_name>                 # single module, e.g. cargo test battle
cargo test --test integration_test       # integration tests only (tests/*.rs)
cargo clippy                             # lint (no custom config, uses defaults)
```

No `rustfmt.toml` or `clippy.toml` exists — defaults apply. No `rust-toolchain.toml` — stable Rust 1.75+ required.

## Workspace layout

```
Cargo.toml          # workspace root: members = ["devi-agent"]
src/                # main server crate (lib + bin)
  lib.rs            # crate root, re-exports all modules
  main.rs           # entrypoint: CLI parse → Core::run()
  core/             # config, logging, setup wizard, panic handler
  game/             # 63 submodules: battle, mob, skill, map, script, etc.
  network/          # TCP server, codec, session management
  protocol/         # rAthena binary packet definitions (~80 packet types)
  storage/          # SQLite/MySQL backend, schema, migration
  error.rs          # thiserror enum (Config, Database, Network, Protocol, Game)
devi-agent/         # separate binary crate (edition 2021)
db/                 # YAML game data (rAthena compatible): items, mobs, skills, drops
config/             # guide.md, server.toml (generated reference)
tests/              # integration tests (9 files)
rathena/            # git submodule — reference rAthena source
```

## Critical conventions

- **`#![allow(non_snake_case)]`** in `src/lib.rs` — rAthena-compatible field names use camelCase. Do NOT rename these to snake_case; they match the binary protocol.
- **rAthena compatibility** is a core goal. Protocol packets, YAML data formats, and script commands must stay compatible with rAthena clients and data files.
- **SQLite is default** (`rusqlite` bundled). MySQL is opt-in via `mysql-backend` feature flag.
- **Tests use in-memory DB**: `Database::open_memory()` — never write test files to disk.

## Runtime config

`deviruchi.toml` is auto-generated on first run via setup wizard. Sections: `[server]`, `[database]`, `[network]`, `[game]`, `[battle]`, `[drop]`, `[exp]`, `[respawn]`, `[logging]`, `[skill]`, `[party]`, `[storage]`, `[chat]`.

## CI / Release

- `.github/workflows/release.yml` triggers on `v*` tags
- Builds for `linux-x64` and `windows-x64`
- `build-release.sh` builds macOS + Windows locally
- `build-linux-docker.sh` and `build-multi-platform.sh` for containerized builds

## Key dependencies

tokio (async runtime), rusqlite (SQLite), serde/serde_yaml/serde_json (serialization), tracing (logging), clap (CLI), argon2 (password hashing), parking_lot (locks), dashmap (concurrent hashmap), pathfinding (A*), tokio-tungstenite (WebSocket).
