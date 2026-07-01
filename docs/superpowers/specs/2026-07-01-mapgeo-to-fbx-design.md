# MapGeo → FBX Converter — Design Spec

## Purpose

A minimal, fast Windows CLI tool that converts League of Legends `.mapgeo` map
geometry files into `.fbx` (ASCII) files for viewing/editing in Blender, Maya,
3ds Max, etc. Built for a friend — not published, no CI, no GitHub remote —
but organized like a clean open-source repo (README, docs, changelog-worthy
history) since it'll be shared as a folder/zip.

Primary UX goal: **double-click or drag-and-drop simplicity**, matching the
pattern already proven in `hematite-v2` (League of Legends skin fixer CLI).
Speed is a hard requirement — no subprocess/FFI bridges, no external SDKs.

## Non-goals

- No texture extraction/decoding/embedding — mapgeo only stores texture
  *paths*, not texture data, so materials are exported as plain named FBX
  Lambert materials with no diffuse texture wired in.
- No skeleton/animation support — `.mapgeo` is static environment geometry
  only; League character skins (`.skn`/`.skl`/`.anm`) are a different format
  entirely and out of scope.
- No WAD reading — the user is expected to hand it an already-extracted
  `.mapgeo` file (or a folder of them).
- No binary FBX output — ASCII only (see Approach below).

## Source data: `.mapgeo` via `rs_mapgeo`

Parsing is provided by the `ritoshark` crate (git dependency, same pin
pattern as `quartz-lib`), which re-exports `rs_mapgeo` under the `mapgeo`
feature:

```toml
ritoshark = { git = "https://github.com/RitoShark/RitoShark-Crates", rev = "<pinned-rev>", features = ["mapgeo"] }
```

Key types consumed (`ritoshark::mapgeo` = `rs_mapgeo`):

- `MapGeometry { version, models: Vec<MapModel>, vertex_descriptions, vertex_buffers, index_buffers, scene_graphs, planar_reflectors, .. }`
- `MapModel { name, transform: Mat4, bounds: Aabb, submeshes: Vec<Submesh>, vertex_buffer_ids, index_buffer_id, vertex_description_id, .. }`
- `Submesh { name, index_start, index_count, min_vertex, max_vertex, .. }` — material name lives on `Submesh.name`
- `VertexDescription { elements: Vec<VertexElement> }` / `VertexElement { name: ElementName, format: ElementFormat }` — describes how to decode the raw `VertexBuffer.data` bytes per vertex

Only `ElementName::Position`, `Normal`, and `Texcoord0` are decoded; other
channels (vertex color, tangent, secondary UVs) are read by the format but
intentionally not carried into the FBX output.

## Output: hand-rolled ASCII FBX 7.4 writer

No mature, actively-maintained pure-Rust crate writes FBX (checked: `fbxcel`
writes binary only and is largely dormant; `fbxcel-dom` and `fbx_direct` are
explicitly unmaintained/deprecated by their own authors). ASCII FBX is simple
enough to hand-write correctly and is universally importable by Blender/Maya/
3ds Max, with no binary footer/offset bookkeeping to get subtly wrong.

Per `.mapgeo` file, the writer emits one `.fbx` scene containing:

- One `Model` + `Geometry` node pair **per `MapModel`** (not merged/flattened)
  — mirrors the source file 1:1. World transform baked from `MapModel.transform`
  into the `Model`'s `Lcl Translation/Rotation/Scaling` properties (decomposed
  from the `Mat4`).
- Per-model `LayerElementMaterial` with `MappingInformationType: "ByPolygon"`,
  `ReferenceInformationType: "IndexToDirect"` — one material index per
  triangle, derived from which `Submesh` range that triangle's indices fall
  into.
- One `Material` node per **unique material name across the whole file**
  (submesh names deduplicated), connected to every `Model` that references it.
- `LayerElementNormal` (`ByPolygonVertex`/`Direct`) and `LayerElementUV`
  (`ByPolygonVertex`/`IndexToDirect`) per geometry.
- No `NodeAttribute`, no `Takes` block (unnecessary for a static, non-animated
  mesh; Blender's importer tolerates their absence).

## CLI / TUX shell (mirrors `hematite-v2`)

Entry-mode detection runs before clap, exactly like
`hematite-cli::detect_entry_mode`:

- **No args** (double-click) → `Interactive`: colored banner splash, numbered
  stdin menu ("Convert a file/folder", "Show info only", "Quit"), prompts for
  a path, "Press Enter to exit" pause before closing.
- **Single existing path, no flags** (drag-and-drop) → `DragDrop`: if it's a
  file, convert it next to itself; if it's a folder, recursively find every
  `*.mapgeo` inside and convert each next to its source, printing a per-file
  progress line and a final aggregate summary (N converted, N failed).
- **Anything else** → `Flagged`: standard `clap` derive parse.

All three converge on one `run_with_cli(Cli)` function, same as hematite-v2's
`run_with_cli`.

### Flags (clap derive)

| Flag | Meaning |
|---|---|
| `<input>` | File or directory (required unless `--info-only` needs it too — always required) |
| `-o, --output <path>` | Output `.fbx` path override (single-file mode only; ignored/invalid in folder mode) |
| `--info-only` | Print the info summary and exit; do not write any `.fbx` |
| `-v, --verbose` | Add a per-model breakdown (name, vertex/triangle count, material) to the info summary |
| `--json` | Machine-readable output for the info summary and the final conversion result |
| `--log-level <quiet|normal|verbose|trace>` | Logging verbosity (kept distinct from `--verbose`, which controls info detail, not log level) |
| `--no-pause` | Skip the "Press Enter to exit" pause (implied by `--json`) |

### Info summary (always printed before converting)

File-level: mapgeo version, model count, total vertices, total triangles,
unique material count, file size. With `--verbose`: additionally one row per
`MapModel` (name, vertex count, triangle count, material list).

### Logging & progress

- `tracing` + `tracing-subscriber`, console + rolling file log (mirrors
  quartz-lib/hematite-v2 conventions).
- `indicatif` progress bar during folder/batch conversion, styled like
  hematite-v2's `ui.rs` (spinner → determinate bar, silent outside Normal
  verbosity/JSON mode).
- `colored` for the splash banner and ✓/✗ per-file result lines.

### Errors

`thiserror` for domain errors (bad magic, unsupported mapgeo version,
unrecognized vertex element/format), `anyhow` at the CLI boundary — same
split used by `rs_mapgeo`/quartz-lib and hematite-v2 respectively.

## Performance

- Pure Rust throughout — no subprocess, no FFI, no external SDK
  installation (deliberately rejecting the existing Quartz
  `xps_fbx_bridge`-style C++/Autodesk-FBX-SDK approach for exactly this
  reason).
- `rayon` parallelizes per-file conversion when a folder is dropped.
- Buffered streaming writes for the FBX text output; no intermediate
  in-memory string concatenation of the whole file.

## Project layout

Standalone folder, not inside the `RitoShark-Crates` workspace (own local git
repo, not pushed anywhere):

```
E:\RitoShark\mapgeo-converter\
  Cargo.toml                 (workspace, resolver = "2")
  README.md
  DEVELOPER.md
  LICENSE                    (matches RitoShark's MIT OR Apache-2.0 convention)
  rust-toolchain.toml
  crates/
    mapgeo2fbx-core/         (mapgeo decode + fbx writer + info, no CLI concerns)
      src/
        mapgeo.rs            (decode MapGeometry -> decoded meshes)
        fbx.rs                (ASCII FBX writer)
        info.rs               (summary struct + formatting)
        error.rs
        lib.rs
    mapgeo2fbx-cli/          (bin crate: entry-mode detection, clap, banner, ui, logging)
      src/
        main.rs
        args.rs
        interactive.rs
        banner.rs
        ui.rs
        logging.rs
  docs/
    superpowers/specs/       (this file)
```

Splitting core (`mapgeo2fbx-core`) from the CLI shell (`mapgeo2fbx-cli`)
keeps the conversion logic testable without any stdin/stdout/clap
concerns, matching how `rs_mapgeo` itself is a pure library crate.

## Testing

- Unit tests in `mapgeo2fbx-core` for vertex decoding (per `ElementFormat`
  variant) and FBX text emission (snapshot-style: assert exact ASCII output
  for a small synthetic `MapGeometry`).
- Integration test converting a real sample `.mapgeo` (if one is available
  under a `Sample-Files/`-style directory, mirroring `RitoShark-Crates`'
  convention) and asserting the output `.fbx` parses back with expected
  vertex/triangle/material counts — skipped cleanly if no sample file is
  present, same convention as `RitoShark-Crates`' real-file tests.
