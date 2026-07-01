# Developer notes

## Layout

- `crates/mapgeo2fbx-core` — pure library: mapgeo vertex decoding (`decode.rs`), the ASCII FBX
  writer (`fbx.rs`), and the info summary (`info.rs`). No CLI/stdin/stdout concerns; fully
  unit-testable in isolation.
- `crates/mapgeo2fbx-cli` — the `mapgeo2fbx` binary: entry-mode detection (double-click / drag-drop
  / flagged), clap argument parsing, the interactive menu, progress UI, and logging.

## Design spec and plan

See `docs/superpowers/specs/2026-07-01-mapgeo-to-fbx-design.md` for the full design rationale
and `docs/superpowers/plans/2026-07-01-mapgeo-to-fbx.md` for the task-by-task implementation
history.

## Running tests

```
cargo test --workspace
```

Core crate tests use synthetic in-memory `MapGeometry` fixtures built the same way
`rs_mapgeo`'s own `tests/smoke.rs` does (hand-assembled OEGM byte layout), so they don't depend
on any external sample files. `crates/mapgeo2fbx-core/tests/real_file_smoke.rs` additionally
exercises the full pipeline against `Sample-Files/bloom.mapgeo` and skips cleanly if that file
is absent.

## Updating the `ritoshark` dependency pin

The workspace pins `ritoshark` to a specific git rev (see `[workspace.dependencies]` in the
root `Cargo.toml`) rather than a crates.io version, matching how `quartz-lib` depends on it.
To pick up new `rs_mapgeo` fixes, bump the `rev` value and re-run `cargo build --workspace`.
