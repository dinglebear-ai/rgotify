---
title: "Rust Build Setup"
created: "2026-07-30"
updated: "2026-07-30"
doc_type: "guide"
status: "active"
owner: "gotify-rmcp"
audience:
  - "contributors"
  - "agents"
scope: "service"
source_of_truth: false
upstream_refs:
  - "https://github.com/dinglebear-ai/soma/blob/main/docs/RUST.md"
last_reviewed: "2026-07-13"
---

# Rust Build Setup

This repo follows the build conventions of the rmcp server family.
The canonical reference is [soma/docs/RUST.md](https://github.com/dinglebear-ai/soma/blob/main/docs/RUST.md).

## System prerequisites

- Rust stable ≥ 1.86 (`rustup update stable`)
- `clang` and `mold` for fast Linux builds: `apt install clang mold`
- `just` command runner (optional): `cargo install just`

## Global Cargo config

Build performance depends on `~/.cargo/config.toml` on the developer's machine.
See [soma/docs/RUST.md](https://github.com/dinglebear-ai/soma/blob/main/docs/RUST.md)
for the expected config (global sccache wrapper, mold linker, profile settings,
and dynamic Cargo job allocation).

## Local `.cargo/config.toml`

This repo has no local `.cargo/config.toml`. All settings (sccache, dynamic jobs,
mold linker, and profile tuning) are inherited from the global config.

This repo has no xtask crate, so no xtask alias is needed.

Refresh repo/plugin binaries explicitly with `just sync-bin`.
