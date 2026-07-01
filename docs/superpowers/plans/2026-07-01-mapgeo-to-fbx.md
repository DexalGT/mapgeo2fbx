# MapGeo → FBX Converter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a minimal, fast Windows CLI (`mapgeo2fbx`) that converts League of Legends `.mapgeo` map geometry files into ASCII `.fbx` scenes, with a double-click/drag-drop-first UX mirroring `hematite-v2`.

**Architecture:** A pure-library core crate (`mapgeo2fbx-core`) owns mapgeo decoding, the hand-rolled ASCII FBX writer, and info summarization — no I/O-prompt/CLI concerns. A separate bin crate (`mapgeo2fbx-cli`) owns entry-mode detection (double-click / drag-drop / flagged), clap parsing, the interactive menu, banner, progress UI, and logging, mirroring `hematite-v2`'s `hematite-cli` layout file-for-file.

**Tech Stack:** Rust 2021, `ritoshark` (git dep, `mapgeo` feature — wraps `rs_mapgeo`), `clap` 4 (derive), `tracing` + `tracing-subscriber`, `indicatif`, `colored`, `anyhow`, `thiserror`, `rayon` (batch folder conversion), `walkdir` (recursive `.mapgeo` discovery).

## Global Constraints

- No subprocess/FFI/external SDKs (no FBX SDK, no C++ bridge) — pure Rust only, per the spec's Performance section.
- FBX output is ASCII 7.4 only — no binary FBX.
- Only `Position`, `Normal`, `Texcoord0` vertex channels are carried into FBX — no vertex color, tangents, or secondary UVs.
- No texture extraction/embedding — materials are plain named FBX Lambert stubs.
- No skeleton/animation support — mapgeo is static geometry only.
- One `Model`+`Geometry` node pair per source `MapModel` — never merge into one mesh.
- One FBX `Material` node per **unique** material name across the whole file (submesh names deduplicated).
- Batch (folder-drop) output: each `.fbx` goes next to its source `.mapgeo`, same base name.
- `ritoshark` git dependency pinned to `rev = "d6af5ac"` (the same rev already used by `quartz-lib`), `features = ["mapgeo"]`.
- Project root: `E:\RitoShark\mapgeo-converter\` (standalone repo, already `git init`'d with user DexalGT <dexalgt@gmail.com>; commits must use that identity, never `-c` overrides, never a co-author trailer).
- Rust edition/toolchain: match `RitoShark-Crates`' `rust-toolchain.toml` (`channel = "1.96.0"`, components `rustfmt`, `clippy`) so the pinned git dependency builds cleanly.

---

## File Structure

```
E:\RitoShark\mapgeo-converter\
  Cargo.toml                        (workspace root, resolver = "2")
  rust-toolchain.toml
  README.md
  DEVELOPER.md
  LICENSE-MIT
  LICENSE-APACHE
  .gitignore
  docs/
    superpowers/
      specs/2026-07-01-mapgeo-to-fbx-design.md   (already written)
      plans/2026-07-01-mapgeo-to-fbx.md            (this file)
  crates/
    mapgeo2fbx-core/
      Cargo.toml
      src/
        lib.rs           (public re-exports)
        error.rs          (Error enum, thiserror)
        decode.rs         (raw MapGeometry -> DecodedMesh Vec, vertex buffer decoding)
        fbx.rs             (ASCII FBX 7.4 writer)
        info.rs            (FileInfo/ModelInfo summary structs + Display/JSON)
      tests/
        decode_tests.rs
        fbx_tests.rs
        info_tests.rs
    mapgeo2fbx-cli/
      Cargo.toml
      src/
        main.rs            (entry-mode detection, top-level flow)
        args.rs             (clap derive Cli struct)
        interactive.rs      (double-click numbered menu)
        banner.rs           (colored splash)
        ui.rs               (indicatif progress + colored result lines)
        logging.rs          (tracing-subscriber setup)
        batch.rs            (folder walk + rayon fan-out + summary)
      tests/
        cli_tests.rs        (assert_cmd integration tests)
```

Core/CLI split matches how `rs_mapgeo` itself is a pure library with no CLI concerns — `mapgeo2fbx-core` is unit-testable with zero stdin/stdout/process dependencies, and `mapgeo2fbx-cli` is the only crate that touches `std::env::args`, `clap`, or the terminal.

---

## Task 1: Workspace scaffold + core crate skeleton

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\Cargo.toml`
- Create: `E:\RitoShark\mapgeo-converter\rust-toolchain.toml`
- Create: `E:\RitoShark\mapgeo-converter\.gitignore`
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\Cargo.toml`
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\src\lib.rs`
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\src\error.rs`

**Interfaces:**
- Produces: `mapgeo2fbx_core::Error` enum (thiserror), `mapgeo2fbx_core::Result<T>` alias — every later task in this crate returns this `Result`.

- [ ] **Step 1: Create the workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
ritoshark = { git = "https://github.com/RitoShark/RitoShark-Crates", rev = "d6af5ac", features = ["mapgeo"] }
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
colored = "2.0"
indicatif = "0.17"
rayon = "1.10"
walkdir = "2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

- [ ] **Step 2: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.96.0"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 3: Create `.gitignore`**

```
/target
Cargo.lock
```

Note: `Cargo.lock` is ignored because this is a bin-producing workspace shared as a folder, not a published library — matches the "keep workspace clean" goal without forcing lockfile churn review. (If the user later wants reproducible builds, this can be flipped; not needed for a hand-off tool.)

- [ ] **Step 4: Create `crates/mapgeo2fbx-core/Cargo.toml`**

```toml
[package]
name = "mapgeo2fbx-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
ritoshark = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
```

- [ ] **Step 5: Create `crates/mapgeo2fbx-core/src/error.rs`**

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse mapgeo file {path}: {source}")]
    MapgeoParse {
        path: PathBuf,
        #[source]
        source: ritoshark::mapgeo::Error,
    },

    #[error("unsupported vertex element format for {element:?}: {format:?}")]
    UnsupportedVertexFormat {
        element: ritoshark::mapgeo::ElementName,
        format: ritoshark::mapgeo::ElementFormat,
    },

    #[error("model {model} references missing vertex buffer id {id}")]
    MissingVertexBuffer { model: String, id: i32 },

    #[error("model {model} references missing index buffer id {id}")]
    MissingIndexBuffer { model: String, id: i32 },

    #[error("failed to write fbx file {path}: {source}")]
    FbxWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 6: Create `crates/mapgeo2fbx-core/src/lib.rs`**

```rust
mod error;

pub use error::{Error, Result};
```

- [ ] **Step 7: Verify the workspace builds**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo build --workspace`
Expected: Compiles successfully (downloads the `ritoshark` git dependency on first run — this may take a minute).

- [ ] **Step 8: Commit**

```bash
cd "E:\RitoShark\mapgeo-converter"
git add Cargo.toml rust-toolchain.toml .gitignore crates/mapgeo2fbx-core
git commit -m "Scaffold workspace and mapgeo2fbx-core error types"
```

---

## Task 2: Vertex buffer decoding

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\src\decode.rs`
- Modify: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\src\lib.rs`
- Test: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\tests\decode_tests.rs`

**Interfaces:**
- Consumes: `mapgeo2fbx_core::Error`/`Result` (Task 1); `ritoshark::mapgeo::{MapGeometry, MapModel, VertexDescription, VertexElement, ElementName, ElementFormat, Submesh}` and `ritoshark::math::{Vec2, Vec3, Mat4}` (external, already vendored).
- Produces:
  - `pub struct DecodedVertex { pub position: Vec3, pub normal: Vec3, pub uv0: Vec2 }`
  - `pub struct DecodedSubmesh { pub name: String, pub triangle_indices: Vec<[u32; 3]> }` (local vertex indices into `DecodedMesh.vertices`, one triangle per entry)
  - `pub struct DecodedMesh { pub name: String, pub vertices: Vec<DecodedVertex>, pub submeshes: Vec<DecodedSubmesh> }`
  - `pub fn decode_geometry(geo: &MapGeometry) -> Result<Vec<DecodedMesh>>` — later tasks (fbx.rs, info.rs) consume this `Vec<DecodedMesh>` as their sole input from mapgeo data.

- [ ] **Step 1: Write the failing test for decoding a minimal synthetic `MapGeometry`**

Create `crates/mapgeo2fbx-core/tests/decode_tests.rs`:

```rust
use mapgeo2fbx_core::decode::decode_geometry;
use ritoshark::mapgeo::{
    AssetChannel, ElementFormat, ElementName, IndexBuffer, MapGeometry, MapModel, Submesh,
    VertexBuffer, VertexDescription, VertexElement, VertexUsage,
};
use ritoshark::math::{Aabb, Mat4, Vec3};

/// One triangle: Position (XyzFloat32) + Normal (XyzFloat32) + Texcoord0 (XyFloat32),
/// laid out in that order — 32 bytes per vertex, 3 vertices, matching how a real mapgeo
/// vertex declaration is described.
fn one_triangle_geometry() -> MapGeometry {
    let elements = vec![
        VertexElement {
            name: ElementName::Position,
            format: ElementFormat::XyzFloat32,
        },
        VertexElement {
            name: ElementName::Normal,
            format: ElementFormat::XyzFloat32,
        },
        VertexElement {
            name: ElementName::Texcoord0,
            format: ElementFormat::XyFloat32,
        },
    ];

    let mut data = Vec::new();
    let verts: [([f32; 3], [f32; 3], [f32; 2]); 3] = [
        ([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0]),
    ];
    for (pos, normal, uv) in verts {
        for v in pos {
            data.extend_from_slice(&v.to_le_bytes());
        }
        for v in normal {
            data.extend_from_slice(&v.to_le_bytes());
        }
        for v in uv {
            data.extend_from_slice(&v.to_le_bytes());
        }
    }

    let model = MapModel {
        name: "MapGeo_Instance_0".to_string(),
        vertex_count: 3,
        vertex_description_id: 0,
        vertex_buffer_ids: vec![0],
        index_count: 3,
        index_buffer_id: 0,
        layer: 0,
        unknown_v18: 0,
        bucket_grid_hash: 0,
        submeshes: vec![Submesh {
            hash: 0,
            name: "Materials/Grass".to_string(),
            index_start: 0,
            index_count: 3,
            min_vertex: 0,
            max_vertex: 2,
        }],
        disable_backface_culling: false,
        bounds: Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0)),
        transform: Mat4::IDENTITY,
        quality: 31,
        layer_transition: 0,
        render_flags: 0,
        point_light: None,
        spherical_harmonics: None,
        baked_light: AssetChannel::empty(),
        stationary_light: AssetChannel::empty(),
        texture_overrides: vec![],
        baked_paint_scale_offset: [0.0; 4],
        baked_paint: None,
    };

    MapGeometry {
        version: 17,
        separate_point_lights: false,
        texture_overrides: vec![],
        vertex_descriptions: vec![VertexDescription {
            usage: VertexUsage::Static,
            elements,
        }],
        vertex_buffers: vec![VertexBuffer { layer: 0, data }],
        index_buffers: vec![IndexBuffer {
            layer: 0,
            indices: vec![0, 1, 2],
        }],
        models: vec![model],
        scene_graphs: vec![],
        planar_reflectors: vec![],
    }
}

#[test]
fn decodes_one_triangle_with_material() {
    let geo = one_triangle_geometry();
    let meshes = decode_geometry(&geo).expect("decode should succeed");

    assert_eq!(meshes.len(), 1);
    let mesh = &meshes[0];
    assert_eq!(mesh.name, "MapGeo_Instance_0");
    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.vertices[1].position, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(mesh.vertices[2].uv0, ritoshark::math::Vec2::new(0.0, 1.0));

    assert_eq!(mesh.submeshes.len(), 1);
    assert_eq!(mesh.submeshes[0].name, "Materials/Grass");
    assert_eq!(mesh.submeshes[0].triangle_indices, vec![[0, 1, 2]]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-core decode_tests`
Expected: FAIL — `error[E0433]: failed to resolve: could not find decode in mapgeo2fbx_core` (module doesn't exist yet).

- [ ] **Step 3: Write `crates/mapgeo2fbx-core/src/decode.rs`**

```rust
use ritoshark::mapgeo::{ElementFormat, ElementName, MapGeometry, MapModel, VertexDescription};
use ritoshark::math::{Vec2, Vec3};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecodedVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv0: Vec2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedSubmesh {
    pub name: String,
    /// Local (per-mesh) vertex indices, one triangle per entry.
    pub triangle_indices: Vec<[u32; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedMesh {
    pub name: String,
    pub vertices: Vec<DecodedVertex>,
    pub submeshes: Vec<DecodedSubmesh>,
}

/// Decodes every placed model in a parsed `.mapgeo` file into renderer-agnostic meshes,
/// applying each model's world transform to positions and normals and grouping triangles
/// by submesh (material) name.
pub fn decode_geometry(geo: &MapGeometry) -> Result<Vec<DecodedMesh>> {
    geo.models.iter().map(|model| decode_model(geo, model)).collect()
}

fn decode_model(geo: &MapGeometry, model: &MapModel) -> Result<DecodedMesh> {
    let vb_id = *model
        .vertex_buffer_ids
        .first()
        .ok_or_else(|| Error::MissingVertexBuffer {
            model: model.name.clone(),
            id: -1,
        })?;
    let vertex_buffer = geo
        .vertex_buffers
        .get(vb_id as usize)
        .ok_or_else(|| Error::MissingVertexBuffer {
            model: model.name.clone(),
            id: vb_id,
        })?;
    let description = geo
        .vertex_descriptions
        .get(model.vertex_description_id as usize)
        .ok_or_else(|| Error::MissingVertexBuffer {
            model: model.name.clone(),
            id: model.vertex_description_id as i32,
        })?;

    let index_buffer = geo
        .index_buffers
        .get(model.index_buffer_id as usize)
        .ok_or_else(|| Error::MissingIndexBuffer {
            model: model.name.clone(),
            id: model.index_buffer_id,
        })?;

    let vertices = decode_vertices(model, description, &vertex_buffer.data)?;

    // Global index buffer -> local (per-model) vertex indices, offset by min_vertex per submesh.
    let submeshes = model
        .submeshes
        .iter()
        .map(|sm| {
            let start = sm.index_start as usize;
            let count = sm.index_count as usize;
            let indices = &index_buffer.indices[start..start + count];
            let triangle_indices = indices
                .chunks_exact(3)
                .map(|tri| [tri[0] as u32, tri[1] as u32, tri[2] as u32])
                .collect();
            DecodedSubmesh {
                name: sm.name.clone(),
                triangle_indices,
            }
        })
        .collect();

    Ok(DecodedMesh {
        name: model.name.clone(),
        vertices,
        submeshes,
    })
}

fn decode_vertices(
    model: &MapModel,
    description: &VertexDescription,
    data: &[u8],
) -> Result<Vec<DecodedVertex>> {
    let stride = description.vertex_size();
    let mut out = Vec::with_capacity(model.vertex_count as usize);

    for i in 0..model.vertex_count as usize {
        let base = i * stride;
        let mut offset = base;
        let mut position = Vec3::ZERO;
        let mut normal = Vec3::ZERO;
        let mut uv0 = Vec2::ZERO;

        for element in &description.elements {
            let size = element.format.byte_size();
            match element.name {
                ElementName::Position => {
                    position = read_vec3(data, offset, element.format)?;
                }
                ElementName::Normal => {
                    normal = read_vec3(data, offset, element.format)?;
                }
                ElementName::Texcoord0 => {
                    uv0 = read_vec2(data, offset, element.format)?;
                }
                _ => {}
            }
            offset += size;
        }

        let transformed_pos = model.transform.transform_point3(position);
        let transformed_normal = model.transform.transform_vector3(normal).normalize_or_zero();

        out.push(DecodedVertex {
            position: transformed_pos,
            normal: transformed_normal,
            uv0,
        });
    }

    Ok(out)
}

fn read_vec3(data: &[u8], offset: usize, format: ElementFormat) -> Result<Vec3> {
    match format {
        ElementFormat::XyzFloat32 => Ok(Vec3::new(
            f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()),
            f32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()),
            f32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap()),
        )),
        other => Err(Error::UnsupportedVertexFormat {
            element: ElementName::Position,
            format: other,
        }),
    }
}

fn read_vec2(data: &[u8], offset: usize, format: ElementFormat) -> Result<Vec2> {
    match format {
        ElementFormat::XyFloat32 => Ok(Vec2::new(
            f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()),
            f32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()),
        )),
        other => Err(Error::UnsupportedVertexFormat {
            element: ElementName::Texcoord0,
            format: other,
        }),
    }
}
```

- [ ] **Step 4: Register the module in `lib.rs`**

Modify `crates/mapgeo2fbx-core/src/lib.rs`:

```rust
mod error;
pub mod decode;

pub use error::{Error, Result};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-core decode_tests`
Expected: PASS — `decodes_one_triangle_with_material ... ok`

- [ ] **Step 6: Commit**

```bash
git add crates/mapgeo2fbx-core/src/decode.rs crates/mapgeo2fbx-core/src/lib.rs crates/mapgeo2fbx-core/tests/decode_tests.rs
git commit -m "Add mapgeo vertex/submesh decoding"
```

---

## Task 3: ASCII FBX writer

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\src\fbx.rs`
- Modify: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\src\lib.rs`
- Test: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\tests\fbx_tests.rs`

**Interfaces:**
- Consumes: `mapgeo2fbx_core::decode::{DecodedMesh, DecodedSubmesh, DecodedVertex}` (Task 2), `mapgeo2fbx_core::Result` (Task 1).
- Produces: `pub fn write_fbx<W: std::io::Write>(writer: &mut W, meshes: &[DecodedMesh]) -> Result<()>` — the CLI's convert command (Task 5) is the only other consumer.

- [ ] **Step 1: Write the failing test asserting exact ASCII output for a two-triangle, two-material mesh**

Create `crates/mapgeo2fbx-core/tests/fbx_tests.rs`:

```rust
use mapgeo2fbx_core::decode::{DecodedMesh, DecodedSubmesh, DecodedVertex};
use mapgeo2fbx_core::fbx::write_fbx;
use ritoshark::math::{Vec2, Vec3};

fn quad_mesh() -> DecodedMesh {
    DecodedMesh {
        name: "MapGeo_Instance_0".to_string(),
        vertices: vec![
            DecodedVertex {
                position: Vec3::new(0.0, 0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
                uv0: Vec2::new(0.0, 0.0),
            },
            DecodedVertex {
                position: Vec3::new(1.0, 0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
                uv0: Vec2::new(1.0, 0.0),
            },
            DecodedVertex {
                position: Vec3::new(1.0, 1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
                uv0: Vec2::new(1.0, 1.0),
            },
        ],
        submeshes: vec![DecodedSubmesh {
            name: "Materials/Grass".to_string(),
            triangle_indices: vec![[0, 1, 2]],
        }],
    }
}

#[test]
fn writes_expected_ascii_for_single_triangle() {
    let meshes = vec![quad_mesh()];
    let mut buf = Vec::new();
    write_fbx(&mut buf, &meshes).expect("write should succeed");
    let text = String::from_utf8(buf).expect("output must be valid utf8");

    assert!(text.contains(r#"FBXHeaderExtension:  {"#));
    assert!(text.contains(r#"Creator: "mapgeo2fbx""#));
    assert!(text.contains(r#"Model: 1000000, "Model::MapGeo_Instance_0", "Mesh""#));
    assert!(text.contains(r#"Geometry: 1000001, "Geometry::MapGeo_Instance_0", "Mesh""#));
    assert!(text.contains("Vertices: *9 {"));
    assert!(text.contains("a: 0,0,0,1,0,0,1,1,0"));
    assert!(text.contains("PolygonVertexIndex: *3 {"));
    assert!(text.contains("a: 0,1,-3"));
    assert!(text.contains(r#"Material: 1000002, "Material::Materials/Grass", """#));
    assert!(text.contains(r#"C: "OO",1000001,1000000"#));
    assert!(text.contains(r#"C: "OO",1000000,0"#));
    assert!(text.contains(r#"C: "OO",1000002,1000000"#));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-core fbx_tests`
Expected: FAIL — `could not find fbx in mapgeo2fbx_core`.

- [ ] **Step 3: Write `crates/mapgeo2fbx-core/src/fbx.rs`**

```rust
use std::collections::HashMap;
use std::io::Write;

use crate::decode::DecodedMesh;
use crate::error::Result;

/// Writes an ASCII FBX 7.4 scene containing one Model+Geometry pair per input mesh, with
/// per-submesh material assignment via a `ByPolygon`/`IndexToDirect` `LayerElementMaterial`.
/// Material nodes are deduplicated by name across the whole scene.
pub fn write_fbx<W: Write>(writer: &mut W, meshes: &[DecodedMesh]) -> Result<()> {
    let mut next_id: i64 = 1_000_000;
    let mut alloc_id = move || {
        let id = next_id;
        next_id += 1;
        id
    };

    // Assign a stable object id to every unique material name across all meshes first, so
    // Connections can reference them regardless of which mesh first used them.
    let mut material_ids: HashMap<String, i64> = HashMap::new();
    for mesh in meshes {
        for submesh in &mesh.submeshes {
            material_ids
                .entry(submesh.name.clone())
                .or_insert_with(&mut alloc_id);
        }
    }

    let mut model_geometry_ids = Vec::with_capacity(meshes.len());
    for mesh in meshes {
        let model_id = alloc_id();
        let geometry_id = alloc_id();
        model_geometry_ids.push((model_id, geometry_id));
    }

    write_header(writer)?;
    write_global_settings(writer)?;
    write_documents(writer)?;
    writeln!(writer, "References:  {{\n}}\n")?;
    write_definitions(writer, meshes.len(), material_ids.len())?;

    writeln!(writer, "Objects:  {{")?;
    for (mesh, (model_id, geometry_id)) in meshes.iter().zip(&model_geometry_ids) {
        write_model(writer, *model_id, &mesh.name)?;
        write_geometry(writer, *geometry_id, mesh, &material_ids)?;
    }
    let mut sorted_materials: Vec<(&String, &i64)> = material_ids.iter().collect();
    sorted_materials.sort_by_key(|(_, id)| **id);
    for (name, id) in sorted_materials {
        write_material(writer, *id, name)?;
    }
    writeln!(writer, "}}\n")?;

    writeln!(writer, "Connections:  {{")?;
    for (mesh, (model_id, geometry_id)) in meshes.iter().zip(&model_geometry_ids) {
        writeln!(writer, "\tC: \"OO\",{geometry_id},{model_id}")?;
        writeln!(writer, "\tC: \"OO\",{model_id},0")?;
        let mut seen = std::collections::HashSet::new();
        for submesh in &mesh.submeshes {
            if seen.insert(&submesh.name) {
                let material_id = material_ids[&submesh.name];
                writeln!(writer, "\tC: \"OO\",{material_id},{model_id}")?;
            }
        }
    }
    writeln!(writer, "}}\n")?;

    Ok(())
}

fn write_header<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "; FBX 7.4.0 project file")?;
    writeln!(writer, "FBXHeaderExtension:  {{")?;
    writeln!(writer, "\tFBXHeaderVersion: 1003")?;
    writeln!(writer, "\tFBXVersion: 7400")?;
    writeln!(writer, "\tCreator: \"mapgeo2fbx\"")?;
    writeln!(writer, "}}\n")?;
    Ok(())
}

fn write_global_settings<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "GlobalSettings:  {{")?;
    writeln!(writer, "\tVersion: 1000")?;
    writeln!(writer, "\tProperties70:  {{")?;
    writeln!(writer, "\t\tP: \"UpAxis\", \"int\", \"Integer\", \"\",1")?;
    writeln!(writer, "\t\tP: \"UpAxisSign\", \"int\", \"Integer\", \"\",1")?;
    writeln!(writer, "\t\tP: \"FrontAxis\", \"int\", \"Integer\", \"\",2")?;
    writeln!(writer, "\t\tP: \"FrontAxisSign\", \"int\", \"Integer\", \"\",1")?;
    writeln!(writer, "\t\tP: \"CoordAxis\", \"int\", \"Integer\", \"\",0")?;
    writeln!(writer, "\t\tP: \"CoordAxisSign\", \"int\", \"Integer\", \"\",1")?;
    writeln!(writer, "\t\tP: \"UnitScaleFactor\", \"double\", \"Number\", \"\",1")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "}}\n")?;
    Ok(())
}

fn write_documents<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "Documents:  {{")?;
    writeln!(writer, "\tCount: 1")?;
    writeln!(writer, "\tDocument: 1000000000, \"\", \"Scene\" {{")?;
    writeln!(writer, "\t\tProperties70:  {{")?;
    writeln!(writer, "\t\t\tP: \"SourceObject\", \"object\", \"\", \"\"")?;
    writeln!(writer, "\t\t\tP: \"ActiveAnimStackName\", \"KString\", \"\", \"\", \"\"")?;
    writeln!(writer, "\t\t}}")?;
    writeln!(writer, "\t\tRootNode: 0")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "}}\n")?;
    Ok(())
}

fn write_definitions<W: Write>(writer: &mut W, model_count: usize, material_count: usize) -> Result<()> {
    writeln!(writer, "Definitions:  {{")?;
    writeln!(writer, "\tVersion: 100")?;
    writeln!(writer, "\tCount: {}", model_count * 2 + material_count)?;
    writeln!(writer, "\tObjectType: \"Model\" {{")?;
    writeln!(writer, "\t\tCount: {model_count}")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "\tObjectType: \"Geometry\" {{")?;
    writeln!(writer, "\t\tCount: {model_count}")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "\tObjectType: \"Material\" {{")?;
    writeln!(writer, "\t\tCount: {material_count}")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "}}\n")?;
    Ok(())
}

fn write_model<W: Write>(writer: &mut W, model_id: i64, name: &str) -> Result<()> {
    writeln!(writer, "\tModel: {model_id}, \"Model::{name}\", \"Mesh\" {{")?;
    writeln!(writer, "\t\tVersion: 232")?;
    writeln!(writer, "\t\tProperties70:  {{")?;
    writeln!(writer, "\t\t\tP: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",0,0,0")?;
    writeln!(writer, "\t\t\tP: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",0,0,0")?;
    writeln!(writer, "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1")?;
    writeln!(writer, "\t\t}}")?;
    writeln!(writer, "\t\tShading: T")?;
    writeln!(writer, "\t\tCulling: \"CullingOff\"")?;
    writeln!(writer, "\t}}\n")?;
    Ok(())
}

fn write_geometry<W: Write>(
    writer: &mut W,
    geometry_id: i64,
    mesh: &DecodedMesh,
    material_ids: &HashMap<String, i64>,
) -> Result<()> {
    // Vertex world transform is already baked into position/normal by decode::decode_geometry,
    // so the Model's Lcl Translation/Rotation/Scaling above stay identity.
    let vertex_count = mesh.vertices.len();

    let mut ordered_material_names: Vec<&String> = Vec::new();
    let mut name_to_local_index: HashMap<&str, u32> = HashMap::new();
    for submesh in &mesh.submeshes {
        if !name_to_local_index.contains_key(submesh.name.as_str()) {
            name_to_local_index.insert(submesh.name.as_str(), ordered_material_names.len() as u32);
            ordered_material_names.push(&submesh.name);
        }
    }
    let _ = material_ids; // material_ids is used by write_fbx's Connections; local index is what LayerElementMaterial needs.

    let mut polygon_vertex_index: Vec<i64> = Vec::new();
    let mut material_per_polygon: Vec<u32> = Vec::new();
    for submesh in &mesh.submeshes {
        let local_material_index = name_to_local_index[submesh.name.as_str()];
        for tri in &submesh.triangle_indices {
            polygon_vertex_index.push(tri[0] as i64);
            polygon_vertex_index.push(tri[1] as i64);
            polygon_vertex_index.push(-(tri[2] as i64) - 1);
            material_per_polygon.push(local_material_index);
        }
    }
    let polygon_count = material_per_polygon.len();

    writeln!(
        writer,
        "\tGeometry: {geometry_id}, \"Geometry::{}\", \"Mesh\" {{",
        mesh.name
    )?;

    let vertex_floats: Vec<String> = mesh
        .vertices
        .iter()
        .flat_map(|v| [v.position.x, v.position.y, v.position.z])
        .map(format_f32)
        .collect();
    writeln!(writer, "\t\tVertices: *{} {{", vertex_count * 3)?;
    writeln!(writer, "\t\t\ta: {}", vertex_floats.join(","))?;
    writeln!(writer, "\t\t}}")?;

    let index_strs: Vec<String> = polygon_vertex_index.iter().map(|i| i.to_string()).collect();
    writeln!(writer, "\t\tPolygonVertexIndex: *{} {{", polygon_vertex_index.len())?;
    writeln!(writer, "\t\t\ta: {}", index_strs.join(","))?;
    writeln!(writer, "\t\t}}")?;
    writeln!(writer, "\t\tGeometryVersion: 124")?;

    // Normals: ByPolygonVertex/Direct, one triplet per (polygon, vertex-in-polygon) — since
    // every polygon here is a triangle, this is 3x the polygon count and mirrors the winding
    // order used above.
    let mut normal_floats: Vec<String> = Vec::new();
    for submesh in &mesh.submeshes {
        for tri in &submesh.triangle_indices {
            for &vi in tri {
                let v = &mesh.vertices[vi as usize];
                normal_floats.push(format_f32(v.normal.x));
                normal_floats.push(format_f32(v.normal.y));
                normal_floats.push(format_f32(v.normal.z));
            }
        }
    }
    writeln!(writer, "\t\tLayerElementNormal: 0 {{")?;
    writeln!(writer, "\t\t\tVersion: 101")?;
    writeln!(writer, "\t\t\tName: \"\"")?;
    writeln!(writer, "\t\t\tMappingInformationType: \"ByPolygonVertex\"")?;
    writeln!(writer, "\t\t\tReferenceInformationType: \"Direct\"")?;
    writeln!(writer, "\t\t\tNormals: *{} {{", normal_floats.len())?;
    writeln!(writer, "\t\t\t\ta: {}", normal_floats.join(","))?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t}}")?;

    // UVs: Direct per-vertex array + IndexToDirect index list matching PolygonVertexIndex order.
    let uv_floats: Vec<String> = mesh
        .vertices
        .iter()
        .flat_map(|v| [v.uv0.x, v.uv0.y])
        .map(format_f32)
        .collect();
    let mut uv_index: Vec<u32> = Vec::new();
    for submesh in &mesh.submeshes {
        for tri in &submesh.triangle_indices {
            uv_index.extend_from_slice(tri);
        }
    }
    let uv_index_strs: Vec<String> = uv_index.iter().map(|i| i.to_string()).collect();
    writeln!(writer, "\t\tLayerElementUV: 0 {{")?;
    writeln!(writer, "\t\t\tVersion: 101")?;
    writeln!(writer, "\t\t\tName: \"\"")?;
    writeln!(writer, "\t\t\tMappingInformationType: \"ByPolygonVertex\"")?;
    writeln!(writer, "\t\t\tReferenceInformationType: \"IndexToDirect\"")?;
    writeln!(writer, "\t\t\tUV: *{} {{", uv_floats.len())?;
    writeln!(writer, "\t\t\t\ta: {}", uv_floats.join(","))?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t\tUVIndex: *{} {{", uv_index_strs.len())?;
    writeln!(writer, "\t\t\t\ta: {}", uv_index_strs.join(","))?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t}}")?;

    let material_index_strs: Vec<String> = material_per_polygon.iter().map(|i| i.to_string()).collect();
    writeln!(writer, "\t\tLayerElementMaterial: 0 {{")?;
    writeln!(writer, "\t\t\tVersion: 101")?;
    writeln!(writer, "\t\t\tName: \"\"")?;
    writeln!(writer, "\t\t\tMappingInformationType: \"ByPolygon\"")?;
    writeln!(writer, "\t\t\tReferenceInformationType: \"IndexToDirect\"")?;
    writeln!(writer, "\t\t\tMaterials: *{polygon_count} {{")?;
    writeln!(writer, "\t\t\t\ta: {}", material_index_strs.join(","))?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t}}")?;

    writeln!(writer, "\t\tLayer: 0 {{")?;
    writeln!(writer, "\t\t\tVersion: 100")?;
    writeln!(writer, "\t\t\tLayerElement:  {{")?;
    writeln!(writer, "\t\t\t\tType: \"LayerElementNormal\"")?;
    writeln!(writer, "\t\t\t\tTypedIndex: 0")?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t\tLayerElement:  {{")?;
    writeln!(writer, "\t\t\t\tType: \"LayerElementMaterial\"")?;
    writeln!(writer, "\t\t\t\tTypedIndex: 0")?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t\tLayerElement:  {{")?;
    writeln!(writer, "\t\t\t\tType: \"LayerElementUV\"")?;
    writeln!(writer, "\t\t\t\tTypedIndex: 0")?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t}}")?;

    writeln!(writer, "\t}}\n")?;
    Ok(())
}

fn write_material<W: Write>(writer: &mut W, material_id: i64, name: &str) -> Result<()> {
    writeln!(writer, "\tMaterial: {material_id}, \"Material::{name}\", \"\" {{")?;
    writeln!(writer, "\t\tVersion: 102")?;
    writeln!(writer, "\t\tShadingModel: \"Lambert\"")?;
    writeln!(writer, "\t\tMultiLayer: 0")?;
    writeln!(writer, "\t\tProperties70:  {{")?;
    writeln!(writer, "\t\t\tP: \"DiffuseColor\", \"Color\", \"\", \"A\",0.8,0.8,0.8")?;
    writeln!(writer, "\t\t}}")?;
    writeln!(writer, "\t}}\n")?;
    Ok(())
}

fn format_f32(v: f32) -> String {
    // FBX ASCII tolerates plain decimal formatting; avoid scientific notation which some
    // importers mis-parse.
    format!("{v}")
}
```

- [ ] **Step 4: Register the module in `lib.rs`**

Modify `crates/mapgeo2fbx-core/src/lib.rs`:

```rust
mod error;
pub mod decode;
pub mod fbx;

pub use error::{Error, Result};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-core fbx_tests`
Expected: PASS — `writes_expected_ascii_for_single_triangle ... ok`

If it fails on exact string matches (e.g. `a: 0,0,0,1,0,0,1,1,0`), print the actual output with
`cargo test -p mapgeo2fbx-core fbx_tests -- --nocapture` and adjust `format_f32`/join logic to
match — the test asserts on `contains(...)` substrings specifically so minor whitespace
differences elsewhere in the file don't cause false failures, but the numeric formatting must
match exactly.

- [ ] **Step 6: Commit**

```bash
git add crates/mapgeo2fbx-core/src/fbx.rs crates/mapgeo2fbx-core/src/lib.rs crates/mapgeo2fbx-core/tests/fbx_tests.rs
git commit -m "Add ASCII FBX 7.4 writer"
```

---

## Task 4: Info summary

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\src\info.rs`
- Modify: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\src\lib.rs`
- Test: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\tests\info_tests.rs`

**Interfaces:**
- Consumes: `ritoshark::mapgeo::MapGeometry` (external), `mapgeo2fbx_core::decode::DecodedMesh` (Task 2, for per-model triangle/vertex counts consistent with what actually gets written to FBX).
- Produces:
  - `#[derive(serde::Serialize)] pub struct ModelInfo { pub name: String, pub vertex_count: usize, pub triangle_count: usize, pub materials: Vec<String> }`
  - `#[derive(serde::Serialize)] pub struct FileInfo { pub version: u32, pub model_count: usize, pub total_vertices: usize, pub total_triangles: usize, pub unique_material_count: usize, pub file_size_bytes: u64, pub models: Vec<ModelInfo> }`
  - `pub fn summarize(geo: &ritoshark::mapgeo::MapGeometry, meshes: &[DecodedMesh], file_size_bytes: u64) -> FileInfo`
  - `impl std::fmt::Display for FileInfo` — human-readable multi-line summary; CLI's info command (Task 6) either prints this Display impl or serializes to JSON depending on `--json`.

- [ ] **Step 1: Write the failing test**

Create `crates/mapgeo2fbx-core/tests/info_tests.rs`:

```rust
use mapgeo2fbx_core::decode::{DecodedMesh, DecodedSubmesh, DecodedVertex};
use mapgeo2fbx_core::info::summarize;
use ritoshark::mapgeo::{
    AssetChannel, IndexBuffer, MapGeometry, MapModel, Submesh, VertexBuffer, VertexDescription,
    VertexUsage,
};
use ritoshark::math::{Aabb, Mat4, Vec2, Vec3};

fn one_model_geo() -> MapGeometry {
    let model = MapModel {
        name: "MapGeo_Instance_0".to_string(),
        vertex_count: 3,
        vertex_description_id: 0,
        vertex_buffer_ids: vec![0],
        index_count: 3,
        index_buffer_id: 0,
        layer: 0,
        unknown_v18: 0,
        bucket_grid_hash: 0,
        submeshes: vec![Submesh {
            hash: 0,
            name: "Materials/Grass".to_string(),
            index_start: 0,
            index_count: 3,
            min_vertex: 0,
            max_vertex: 2,
        }],
        disable_backface_culling: false,
        bounds: Aabb::new(Vec3::ZERO, Vec3::ONE),
        transform: Mat4::IDENTITY,
        quality: 31,
        layer_transition: 0,
        render_flags: 0,
        point_light: None,
        spherical_harmonics: None,
        baked_light: AssetChannel::empty(),
        stationary_light: AssetChannel::empty(),
        texture_overrides: vec![],
        baked_paint_scale_offset: [0.0; 4],
        baked_paint: None,
    };

    MapGeometry {
        version: 17,
        separate_point_lights: false,
        texture_overrides: vec![],
        vertex_descriptions: vec![VertexDescription {
            usage: VertexUsage::Static,
            elements: vec![],
        }],
        vertex_buffers: vec![VertexBuffer {
            layer: 0,
            data: vec![],
        }],
        index_buffers: vec![IndexBuffer {
            layer: 0,
            indices: vec![0, 1, 2],
        }],
        models: vec![model],
        scene_graphs: vec![],
        planar_reflectors: vec![],
    }
}

fn one_decoded_mesh() -> Vec<DecodedMesh> {
    vec![DecodedMesh {
        name: "MapGeo_Instance_0".to_string(),
        vertices: vec![
            DecodedVertex {
                position: Vec3::ZERO,
                normal: Vec3::Z,
                uv0: Vec2::ZERO,
            };
            3
        ],
        submeshes: vec![DecodedSubmesh {
            name: "Materials/Grass".to_string(),
            triangle_indices: vec![[0, 1, 2]],
        }],
    }]
}

#[test]
fn summarizes_file_and_model_totals() {
    let geo = one_model_geo();
    let meshes = one_decoded_mesh();
    let info = summarize(&geo, &meshes, 4096);

    assert_eq!(info.version, 17);
    assert_eq!(info.model_count, 1);
    assert_eq!(info.total_vertices, 3);
    assert_eq!(info.total_triangles, 1);
    assert_eq!(info.unique_material_count, 1);
    assert_eq!(info.file_size_bytes, 4096);
    assert_eq!(info.models.len(), 1);
    assert_eq!(info.models[0].name, "MapGeo_Instance_0");
    assert_eq!(info.models[0].vertex_count, 3);
    assert_eq!(info.models[0].triangle_count, 1);
    assert_eq!(info.models[0].materials, vec!["Materials/Grass".to_string()]);

    let rendered = format!("{info}");
    assert!(rendered.contains("version: 17"));
    assert!(rendered.contains("models: 1"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-core info_tests`
Expected: FAIL — `could not find info in mapgeo2fbx_core`.

- [ ] **Step 3: Write `crates/mapgeo2fbx-core/src/info.rs`**

```rust
use std::collections::HashSet;
use std::fmt;

use ritoshark::mapgeo::MapGeometry;

use crate::decode::DecodedMesh;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub materials: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileInfo {
    pub version: u32,
    pub model_count: usize,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub unique_material_count: usize,
    pub file_size_bytes: u64,
    pub models: Vec<ModelInfo>,
}

/// Builds a summary from the raw parsed geometry (for the version/file-size header) plus the
/// already-decoded meshes (for vertex/triangle/material counts, so the numbers match exactly
/// what the FBX writer will emit).
pub fn summarize(geo: &MapGeometry, meshes: &[DecodedMesh], file_size_bytes: u64) -> FileInfo {
    let mut all_materials: HashSet<&str> = HashSet::new();
    let mut total_vertices = 0usize;
    let mut total_triangles = 0usize;

    let models: Vec<ModelInfo> = meshes
        .iter()
        .map(|mesh| {
            let mut model_materials: Vec<String> = Vec::new();
            let mut seen = HashSet::new();
            let mut triangle_count = 0usize;
            for submesh in &mesh.submeshes {
                triangle_count += submesh.triangle_indices.len();
                if seen.insert(submesh.name.as_str()) {
                    model_materials.push(submesh.name.clone());
                }
                all_materials.insert(submesh.name.as_str());
            }
            total_vertices += mesh.vertices.len();
            total_triangles += triangle_count;

            ModelInfo {
                name: mesh.name.clone(),
                vertex_count: mesh.vertices.len(),
                triangle_count,
                materials: model_materials,
            }
        })
        .collect();

    FileInfo {
        version: geo.version,
        model_count: models.len(),
        total_vertices,
        total_triangles,
        unique_material_count: all_materials.len(),
        file_size_bytes,
        models,
    }
}

impl fmt::Display for FileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "mapgeo version: {}", self.version)?;
        writeln!(f, "models: {}", self.model_count)?;
        writeln!(f, "total vertices: {}", self.total_vertices)?;
        writeln!(f, "total triangles: {}", self.total_triangles)?;
        writeln!(f, "unique materials: {}", self.unique_material_count)?;
        write!(f, "file size: {} bytes", self.file_size_bytes)?;
        for model in &self.models {
            writeln!(f)?;
            write!(
                f,
                "  - {}: {} verts, {} tris, materials: [{}]",
                model.name,
                model.vertex_count,
                model.triangle_count,
                model.materials.join(", ")
            )?;
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Register the module in `lib.rs`**

Modify `crates/mapgeo2fbx-core/src/lib.rs`:

```rust
mod error;
pub mod decode;
pub mod fbx;
pub mod info;

pub use error::{Error, Result};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-core info_tests`
Expected: PASS — `summarizes_file_and_model_totals ... ok`

- [ ] **Step 6: Run the full core test suite**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-core`
Expected: All tests pass (`decode_tests`, `fbx_tests`, `info_tests`).

- [ ] **Step 7: Commit**

```bash
git add crates/mapgeo2fbx-core/src/info.rs crates/mapgeo2fbx-core/src/lib.rs crates/mapgeo2fbx-core/tests/info_tests.rs
git commit -m "Add mapgeo file/model info summary"
```

---

## Task 5: CLI scaffold — args, logging, banner, single-file conversion

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\Cargo.toml`
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\args.rs`
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\logging.rs`
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\banner.rs`
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\main.rs`
- Test: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\tests\cli_tests.rs`

**Interfaces:**
- Consumes: `mapgeo2fbx_core::{decode::decode_geometry, fbx::write_fbx, info::summarize, Error, Result}` (Tasks 2–4); `ritoshark::mapgeo::MapGeometry`, `ritoshark::io::Parse`, `ritoshark::io::Serialize` (external).
- Produces:
  - `pub struct Cli { pub input: PathBuf, pub output: Option<PathBuf>, pub info_only: bool, pub verbose: bool, pub json: bool, pub log_level: LogLevel, pub no_pause: bool }` (clap derive) — Task 6 (batch) and Task 7 (interactive/drag-drop) both construct/consume this.
  - `pub fn convert_one_file(input: &Path, output_override: Option<&Path>) -> anyhow::Result<mapgeo2fbx_core::info::FileInfo>` — converts a single `.mapgeo` to `.fbx` next to it (or to `output_override`), returns the info summary for the caller to print/aggregate. This is the single conversion primitive Task 6's batch mode calls per-file.

- [ ] **Step 1: Create `crates/mapgeo2fbx-cli/Cargo.toml`**

```toml
[package]
name = "mapgeo2fbx-cli"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "mapgeo2fbx"
path = "src/main.rs"

[dependencies]
mapgeo2fbx-core = { path = "../mapgeo2fbx-core" }
ritoshark = { workspace = true }
clap = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
colored = { workspace = true }
indicatif = { workspace = true }
rayon = { workspace = true }
walkdir = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 2: Create `crates/mapgeo2fbx-cli/src/args.rs`**

```rust
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "mapgeo2fbx")]
#[command(about = "Convert League of Legends .mapgeo map geometry to ASCII .fbx")]
#[command(version)]
pub struct Cli {
    /// Input .mapgeo file, or a directory to recursively convert every .mapgeo inside.
    pub input: PathBuf,

    /// Output .fbx path override. Only valid when `input` is a single file.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Print the info summary and exit without writing any .fbx.
    #[arg(long)]
    pub info_only: bool,

    /// Include a per-model breakdown in the info summary.
    #[arg(short, long)]
    pub verbose: bool,

    /// Machine-readable JSON output for the info summary and conversion result.
    #[arg(long)]
    pub json: bool,

    /// Logging verbosity, independent of --verbose (which controls info detail).
    #[arg(long, value_enum, default_value = "normal")]
    pub log_level: LogLevel,

    /// Skip the "Press Enter to exit" pause at the end.
    #[arg(long)]
    pub no_pause: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LogLevel {
    Quiet,
    Normal,
    Verbose,
    Trace,
}
```

- [ ] **Step 3: Create `crates/mapgeo2fbx-cli/src/logging.rs`**

```rust
use tracing::Level;
use tracing_subscriber::EnvFilter;

use crate::args::LogLevel;

/// Initializes a console tracing subscriber at the requested level. Safe to call once at
/// startup; a second call is a silent no-op (matches `hematite-cli`'s `logging::init`).
pub fn init(log_level: LogLevel, json: bool) {
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    let level = match log_level {
        LogLevel::Quiet => Level::ERROR,
        LogLevel::Normal => Level::WARN,
        LogLevel::Verbose => Level::DEBUG,
        LogLevel::Trace => Level::TRACE,
    };

    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    let result = if json {
        tracing_subscriber::fmt().json().with_env_filter(filter).try_init()
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .try_init()
    };
    let _ = result;
}
```

- [ ] **Step 4: Create `crates/mapgeo2fbx-cli/src/banner.rs`**

```rust
use colored::Colorize;

const TAGLINE: &str = "MapGeo -> FBX converter";

/// Prints the splash to stderr (stdout stays clean for --json / piped output).
pub fn print() {
    eprintln!();
    eprintln!("{}", "  mapgeo2fbx".bright_cyan().bold());
    eprintln!(
        "  {}    {}",
        TAGLINE.bright_white(),
        format!("v{}", env!("CARGO_PKG_VERSION")).bright_black()
    );
    eprintln!(
        "  {} {}",
        "tip:".bright_black(),
        "drag a .mapgeo file or a folder onto this exe to convert it"
            .bright_black()
            .italic()
    );
    eprintln!();
}
```

- [ ] **Step 5: Create `crates/mapgeo2fbx-cli/src/main.rs` with the single-file conversion primitive**

```rust
mod args;
mod banner;
mod logging;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use args::Cli;
use clap::Parser;
use colored::Colorize;
use mapgeo2fbx_core::decode::decode_geometry;
use mapgeo2fbx_core::fbx::write_fbx;
use mapgeo2fbx_core::info::{summarize, FileInfo};
use ritoshark::io::{Parse, Serialize as _};
use ritoshark::mapgeo::MapGeometry;

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let cli = Cli::parse_from(raw);

    logging::init(cli.log_level, cli.json);
    banner::print();

    let result = run(&cli);
    if let Err(ref e) = result {
        eprintln!("{} {e:#}", "error:".bright_red().bold());
    }

    if !cli.no_pause && !cli.json {
        eprintln!();
        eprintln!("Press Enter to exit...");
        let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0u8]);
    }

    if result.is_err() {
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    if !cli.input.exists() {
        anyhow::bail!("input path does not exist: {}", cli.input.display());
    }

    if cli.input.is_file() {
        let info = if cli.info_only {
            load_info(&cli.input)?
        } else {
            convert_one_file(&cli.input, cli.output.as_deref())?
        };
        print_info(&info, cli.json, cli.verbose);
        Ok(())
    } else {
        anyhow::bail!(
            "'{}' is a directory — batch conversion is handled by the CLI's batch mode, not implemented in this task yet",
            cli.input.display()
        );
    }
}

/// Parses a `.mapgeo` file and returns its info summary without writing any `.fbx`.
fn load_info(path: &Path) -> Result<FileInfo> {
    let geo = parse_mapgeo(path)?;
    let meshes = decode_geometry(&geo).with_context(|| format!("decoding {}", path.display()))?;
    let file_size = fs::metadata(path)
        .with_context(|| format!("reading metadata for {}", path.display()))?
        .len();
    Ok(summarize(&geo, &meshes, file_size))
}

/// Converts a single `.mapgeo` file to `.fbx`, writing next to the source unless
/// `output_override` is given. Returns the info summary of what was converted.
pub fn convert_one_file(input: &Path, output_override: Option<&Path>) -> Result<FileInfo> {
    let geo = parse_mapgeo(input)?;
    let meshes = decode_geometry(&geo).with_context(|| format!("decoding {}", input.display()))?;
    let file_size = fs::metadata(input)
        .with_context(|| format!("reading metadata for {}", input.display()))?
        .len();
    let info = summarize(&geo, &meshes, file_size);

    let output_path = output_override
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("fbx"));

    let mut file = fs::File::create(&output_path)
        .with_context(|| format!("creating {}", output_path.display()))?;
    write_fbx(&mut file, &meshes).with_context(|| format!("writing {}", output_path.display()))?;

    tracing::info!(input = %input.display(), output = %output_path.display(), "converted");
    Ok(info)
}

fn parse_mapgeo(path: &Path) -> Result<MapGeometry> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    MapGeometry::from_bytes(&bytes).with_context(|| format!("parsing {}", path.display()))
}

fn print_info(info: &FileInfo, json: bool, verbose: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(info).expect("FileInfo serializes"));
        return;
    }
    if verbose {
        println!("{info}");
    } else {
        println!(
            "mapgeo version: {} | models: {} | vertices: {} | triangles: {} | materials: {} | size: {} bytes",
            info.version,
            info.model_count,
            info.total_vertices,
            info.total_triangles,
            info.unique_material_count,
            info.file_size_bytes
        );
    }
}
```

- [ ] **Step 6: Write the failing integration test**

Create `crates/mapgeo2fbx-cli/tests/cli_tests.rs`. This test builds a minimal on-disk `.mapgeo`
file using the same byte-layout approach as `rs_mapgeo`'s own `tests/smoke.rs` fixture (a
single-vertex, one-model, zero-submesh file is enough to prove the CLI round-trips end to end):

```rust
use std::fs;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

/// Builds the smallest valid OEGM v17 file: one Position-only vertex declaration, one
/// vertex/index buffer, one model, no submeshes. Mirrors `rs_mapgeo`'s own `tests/smoke.rs`
/// fixture so the byte layout is known-good against the real parser.
fn minimal_v17_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"OEGM");
    b.extend_from_slice(&17u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes()); // texture overrides

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 vertex decl
    b.extend_from_slice(&0u32.to_le_bytes()); // usage = Static
    b.extend_from_slice(&1u32.to_le_bytes()); // element count
    b.extend_from_slice(&0u32.to_le_bytes()); // name = Position
    b.extend_from_slice(&2u32.to_le_bytes()); // format = XYZ_Float32
    for _ in 0..14 {
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&3u32.to_le_bytes());
    }

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 vertex buffer
    b.push(0u8);
    b.extend_from_slice(&12u32.to_le_bytes());
    b.extend_from_slice(&1.0f32.to_le_bytes());
    b.extend_from_slice(&2.0f32.to_le_bytes());
    b.extend_from_slice(&3.0f32.to_le_bytes());

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 index buffer
    b.push(0u8);
    b.extend_from_slice(&6u32.to_le_bytes());
    for i in 0u16..3 {
        b.extend_from_slice(&i.to_le_bytes());
    }

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 model
    b.extend_from_slice(&1u32.to_le_bytes()); // vertex count
    b.extend_from_slice(&1u32.to_le_bytes()); // vertex buffer count
    b.extend_from_slice(&0u32.to_le_bytes()); // vertex desc id
    b.extend_from_slice(&0i32.to_le_bytes()); // vertex buffer id
    b.extend_from_slice(&3u32.to_le_bytes()); // index count
    b.extend_from_slice(&0i32.to_le_bytes()); // index buffer id
    b.push(0u8); // layer
    b.extend_from_slice(&0u32.to_le_bytes()); // bucket grid hash
    b.extend_from_slice(&0u32.to_le_bytes()); // submesh count
    b.push(0u8); // disable backface culling
    for v in [0.0f32, 0.0, 0.0, 1.0, 1.0, 1.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    let identity: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    for v in identity {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.push(31u8); // quality
    b.push(0u8); // layer transition behavior
    b.extend_from_slice(&0u16.to_le_bytes()); // render flags
    b.extend_from_slice(&0u32.to_le_bytes()); // baked light path
    for v in [1.0f32, 1.0, 0.0, 0.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0u32.to_le_bytes()); // stationary light path
    for v in [0.0f32, 0.0, 0.0, 0.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0u32.to_le_bytes()); // model texture overrides
    for v in [0.0f32, 0.0, 0.0, 0.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 disabled scene graph
    b.extend_from_slice(&0u32.to_le_bytes());
    for v in [0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0u16.to_le_bytes());
    b.push(1u8); // is_disabled
    b.push(0u8);
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());

    b.extend_from_slice(&0u32.to_le_bytes()); // 0 planar reflectors

    b
}

#[test]
fn converts_minimal_mapgeo_to_fbx() {
    let dir = tempdir().expect("tempdir");
    let input_path = dir.path().join("test.mapgeo");
    fs::write(&input_path, minimal_v17_bytes()).expect("write fixture");

    Command::cargo_bin("mapgeo2fbx")
        .expect("binary exists")
        .arg(&input_path)
        .arg("--no-pause")
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"model_count\": 1"));

    let output_path = dir.path().join("test.fbx");
    assert!(output_path.exists(), "expected test.fbx to be written next to test.mapgeo");
    let fbx_text = fs::read_to_string(&output_path).expect("read fbx output");
    assert!(fbx_text.contains("FBXHeaderExtension"));
}

#[test]
fn info_only_does_not_write_fbx() {
    let dir = tempdir().expect("tempdir");
    let input_path = dir.path().join("test.mapgeo");
    fs::write(&input_path, minimal_v17_bytes()).expect("write fixture");

    Command::cargo_bin("mapgeo2fbx")
        .expect("binary exists")
        .arg(&input_path)
        .arg("--info-only")
        .arg("--no-pause")
        .arg("--json")
        .assert()
        .success();

    let output_path = dir.path().join("test.fbx");
    assert!(!output_path.exists(), "--info-only must not write an fbx file");
}
```

- [ ] **Step 7: Run test to verify it fails**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-cli`
Expected: FAIL to compile initially if `Cargo.toml`/module wiring is incomplete — resolve any
compile errors first, then expect the tests to fail at assertion time if `print_info`'s JSON
key names don't match `"model_count"` (they do, since `FileInfo` derives `serde::Serialize` with
default field-name casing — verify the actual key is `model_count`, not `modelCount`, since serde
defaults to the Rust field name unless a rename attribute is present, which `FileInfo` does not
have).

- [ ] **Step 8: Write minimal fixes and re-run until green**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-cli`
Expected: PASS — both `converts_minimal_mapgeo_to_fbx` and `info_only_does_not_write_fbx` succeed.

- [ ] **Step 9: Commit**

```bash
git add crates/mapgeo2fbx-cli
git commit -m "Add CLI single-file conversion and info-only mode"
```

---

## Task 6: Batch (folder) conversion

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\ui.rs`
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\batch.rs`
- Modify: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\main.rs`
- Test: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\tests\cli_tests.rs` (append)

**Interfaces:**
- Consumes: `convert_one_file` (Task 5, moved from `main.rs` into `batch.rs` as a shared helper — see Step 2), `walkdir::WalkDir`, `rayon::prelude::*`.
- Produces: `pub struct BatchSummary { pub converted: usize, pub failed: Vec<(PathBuf, String)> }`, `pub fn convert_folder(root: &Path, ui: &ui::UiReporter) -> BatchSummary` — `main.rs`'s `run()` calls this when `cli.input.is_dir()`.

- [ ] **Step 1: Create `crates/mapgeo2fbx-cli/src/ui.rs`**

```rust
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Live,
    Silent,
}

impl Mode {
    pub fn from_json(json: bool) -> Self {
        if json {
            Mode::Silent
        } else {
            Mode::Live
        }
    }
}

#[derive(Clone)]
pub struct UiReporter {
    bar: Option<ProgressBar>,
}

impl UiReporter {
    pub fn new(mode: Mode, total: u64) -> Self {
        let bar = match mode {
            Mode::Silent => None,
            Mode::Live => {
                let pb = ProgressBar::new(total);
                pb.set_style(
                    ProgressStyle::with_template(
                        "  [{bar:30.cyan/black}] {pos}/{len} {msg}",
                    )
                    .expect("hard-coded progress style is valid")
                    .progress_chars("█▉▊▋▌▍▎  "),
                );
                Some(pb)
            }
        };
        Self { bar }
    }

    pub fn tick(&self) {
        if let Some(bar) = &self.bar {
            bar.inc(1);
        }
    }

    pub fn converted(&self, name: &str) {
        let line = format!("  {} {}", "✓".bright_green().bold(), name);
        match &self.bar {
            Some(bar) => bar.println(line),
            None => eprintln!("{line}"),
        }
    }

    pub fn failed(&self, name: &str, error: &str) {
        let line = format!("  {} {} — {}", "✗".bright_red().bold(), name, error.bright_black());
        match &self.bar {
            Some(bar) => bar.println(line),
            None => eprintln!("{line}"),
        }
    }

    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }
}
```

- [ ] **Step 2: Move the conversion primitive into `batch.rs` and add folder conversion**

Create `crates/mapgeo2fbx-cli/src/batch.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use mapgeo2fbx_core::decode::decode_geometry;
use mapgeo2fbx_core::fbx::write_fbx;
use mapgeo2fbx_core::info::{summarize, FileInfo};
use rayon::prelude::*;
use ritoshark::io::Parse;
use ritoshark::mapgeo::MapGeometry;
use walkdir::WalkDir;

use crate::ui::UiReporter;

pub struct BatchSummary {
    pub converted: usize,
    pub failed: Vec<(PathBuf, String)>,
}

fn parse_mapgeo(path: &Path) -> Result<MapGeometry> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    MapGeometry::from_bytes(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Parses a `.mapgeo` file and returns its info summary without writing any `.fbx`.
pub fn load_info(path: &Path) -> Result<FileInfo> {
    let geo = parse_mapgeo(path)?;
    let meshes = decode_geometry(&geo).with_context(|| format!("decoding {}", path.display()))?;
    let file_size = fs::metadata(path)
        .with_context(|| format!("reading metadata for {}", path.display()))?
        .len();
    Ok(summarize(&geo, &meshes, file_size))
}

/// Converts a single `.mapgeo` file to `.fbx`, writing next to the source unless
/// `output_override` is given. Returns the info summary of what was converted.
pub fn convert_one_file(input: &Path, output_override: Option<&Path>) -> Result<FileInfo> {
    let geo = parse_mapgeo(input)?;
    let meshes = decode_geometry(&geo).with_context(|| format!("decoding {}", input.display()))?;
    let file_size = fs::metadata(input)
        .with_context(|| format!("reading metadata for {}", input.display()))?
        .len();
    let info = summarize(&geo, &meshes, file_size);

    let output_path = output_override
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("fbx"));

    let mut file = fs::File::create(&output_path)
        .with_context(|| format!("creating {}", output_path.display()))?;
    write_fbx(&mut file, &meshes).with_context(|| format!("writing {}", output_path.display()))?;

    tracing::info!(input = %input.display(), output = %output_path.display(), "converted");
    Ok(info)
}

/// Recursively finds every `.mapgeo` under `root` and converts each next to its source,
/// in parallel. Errors on individual files are collected rather than aborting the whole batch.
pub fn convert_folder(root: &Path, ui: &UiReporter) -> BatchSummary {
    let files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("mapgeo"))
                .unwrap_or(false)
        })
        .collect();

    let failed: Mutex<Vec<(PathBuf, String)>> = Mutex::new(Vec::new());
    let converted_count = Mutex::new(0usize);

    files.par_iter().for_each(|path| {
        match convert_one_file(path, None) {
            Ok(_) => {
                *converted_count.lock().expect("lock poisoned") += 1;
                ui.converted(&path.display().to_string());
            }
            Err(e) => {
                failed.lock().expect("lock poisoned").push((path.clone(), format!("{e:#}")));
                ui.failed(&path.display().to_string(), &format!("{e:#}"));
            }
        }
        ui.tick();
    });

    BatchSummary {
        converted: *converted_count.lock().expect("lock poisoned"),
        failed: failed.into_inner().expect("lock poisoned"),
    }
}
```

- [ ] **Step 3: Update `main.rs` to use `batch.rs` and dispatch on file vs. directory**

Modify `crates/mapgeo2fbx-cli/src/main.rs` — replace the whole file:

```rust
mod args;
mod banner;
mod batch;
mod logging;
mod ui;

use anyhow::Result;
use args::Cli;
use clap::Parser;
use colored::Colorize;
use mapgeo2fbx_core::info::FileInfo;

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let cli = Cli::parse_from(raw);

    logging::init(cli.log_level, cli.json);
    banner::print();

    let result = run(&cli);
    if let Err(ref e) = result {
        eprintln!("{} {e:#}", "error:".bright_red().bold());
    }

    if !cli.no_pause && !cli.json {
        eprintln!();
        eprintln!("Press Enter to exit...");
        let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0u8]);
    }

    if result.is_err() {
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    if !cli.input.exists() {
        anyhow::bail!("input path does not exist: {}", cli.input.display());
    }

    if cli.input.is_file() {
        let info = if cli.info_only {
            batch::load_info(&cli.input)?
        } else {
            batch::convert_one_file(&cli.input, cli.output.as_deref())?
        };
        print_info(&info, cli.json, cli.verbose);
        Ok(())
    } else {
        if cli.output.is_some() {
            anyhow::bail!("--output is only valid when converting a single file, not a directory");
        }
        let mode = ui::Mode::from_json(cli.json);
        let file_count = walkdir::WalkDir::new(&cli.input)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("mapgeo"))
                    .unwrap_or(false)
            })
            .count() as u64;

        let reporter = ui::UiReporter::new(mode, file_count);
        let summary = batch::convert_folder(&cli.input, &reporter);
        reporter.finish();

        if cli.json {
            let payload = serde_json::json!({
                "converted": summary.converted,
                "failed": summary.failed.iter().map(|(p, e)| serde_json::json!({
                    "path": p.display().to_string(),
                    "error": e,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
        } else {
            println!(
                "\nconverted {} file(s), {} failed",
                summary.converted,
                summary.failed.len()
            );
        }

        if !summary.failed.is_empty() {
            anyhow::bail!("{} file(s) failed to convert", summary.failed.len());
        }
        Ok(())
    }
}

fn print_info(info: &FileInfo, json: bool, verbose: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(info).expect("FileInfo serializes"));
        return;
    }
    if verbose {
        println!("{info}");
    } else {
        println!(
            "mapgeo version: {} | models: {} | vertices: {} | triangles: {} | materials: {} | size: {} bytes",
            info.version,
            info.model_count,
            info.total_vertices,
            info.total_triangles,
            info.unique_material_count,
            info.file_size_bytes
        );
    }
}
```

- [ ] **Step 4: Write the failing test for folder conversion**

Append to `crates/mapgeo2fbx-cli/tests/cli_tests.rs` (reuses `minimal_v17_bytes()` already
defined in that file from Task 5):

```rust
#[test]
fn converts_folder_recursively() {
    let dir = tempdir().expect("tempdir");
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).expect("mkdir nested");

    fs::write(dir.path().join("a.mapgeo"), minimal_v17_bytes()).expect("write a");
    fs::write(nested.join("b.mapgeo"), minimal_v17_bytes()).expect("write b");

    Command::cargo_bin("mapgeo2fbx")
        .expect("binary exists")
        .arg(dir.path())
        .arg("--no-pause")
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"converted\": 2"));

    assert!(dir.path().join("a.fbx").exists());
    assert!(nested.join("b.fbx").exists());
}
```

- [ ] **Step 5: Run test to verify it fails**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-cli converts_folder_recursively`
Expected: FAIL if `main.rs`/`batch.rs` wiring has a compile error — fix any, then confirm the
test fails for the right reason (missing feature) if there's a logic gap before Step 3's edit
lands; since Step 3 already implements the feature, this step's real purpose is to catch
mistakes in the implementation.

- [ ] **Step 6: Run test to verify it passes**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-cli`
Expected: PASS — all four CLI integration tests (`converts_minimal_mapgeo_to_fbx`,
`info_only_does_not_write_fbx`, `converts_folder_recursively`, plus any others) succeed.

- [ ] **Step 7: Commit**

```bash
git add crates/mapgeo2fbx-cli
git commit -m "Add recursive folder batch conversion with progress UI"
```

---

## Task 7: Entry-mode detection (double-click / drag-drop / flagged) + interactive menu

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\interactive.rs`
- Modify: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\src\main.rs`
- Test: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-cli\tests\cli_tests.rs` (append)

**Interfaces:**
- Consumes: `args::Cli`, `args::LogLevel`, `batch::{convert_one_file, convert_folder, load_info}`, `ui::{Mode, UiReporter}` (all from Tasks 5–6).
- Produces: `enum EntryMode { Interactive, DragDrop(PathBuf), Flagged }`, `fn detect_entry_mode(raw: &[String]) -> EntryMode` in `main.rs` — this is the top-level dispatch the binary's `main()` uses; no other task consumes it, it's the final integration point.

- [ ] **Step 1: Create `crates/mapgeo2fbx-cli/src/interactive.rs`**

```rust
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;

use crate::args::{Cli, LogLevel};

/// Runs the numbered menu loop shown on a pure double-click (no args). Converges on the same
/// `crate::run` used by the flagged and drag-drop paths so behavior stays consistent.
pub fn run() -> Result<()> {
    crate::banner::print();

    loop {
        let action = prompt_action()?;
        match action {
            Action::Convert => {
                if let Some(path) = prompt_path("Drop a .mapgeo file or folder here (or paste a path)")? {
                    let cli = baseline_cli(path);
                    if let Err(e) = crate::run(&cli) {
                        eprintln!("{} {e:#}", "error:".bright_red().bold());
                    }
                }
            }
            Action::InfoOnly => {
                if let Some(path) = prompt_path("Drop a .mapgeo file here to inspect (or paste a path)")? {
                    let mut cli = baseline_cli(path);
                    cli.info_only = true;
                    cli.verbose = true;
                    if let Err(e) = crate::run(&cli) {
                        eprintln!("{} {e:#}", "error:".bright_red().bold());
                    }
                }
            }
            Action::Quit => break,
        }

        if !prompt_yes_no("Do another?", true)? {
            break;
        }
    }

    eprintln!("\n  {}\n", "bye!".bright_cyan());
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Convert,
    InfoOnly,
    Quit,
}

const MENU: &[(Action, &str, &str)] = &[
    (Action::Convert, "Convert a .mapgeo (or folder)", "writes .fbx next to each source"),
    (Action::InfoOnly, "Show info only", "inspect a .mapgeo without converting"),
    (Action::Quit, "Quit", ""),
];

fn prompt_action() -> Result<Action> {
    eprintln!("  {}", "What do you want to do?".bright_white().bold());
    eprintln!();
    for (i, (_, label, hint)) in MENU.iter().enumerate() {
        if hint.is_empty() {
            eprintln!("    [{}]  {}", i + 1, label);
        } else {
            eprintln!("    [{}]  {} {}", i + 1, label, format!("— {hint}").bright_black());
        }
    }
    eprintln!();

    loop {
        let raw = read_line(&format!("  choice (1-{}): ", MENU.len()))?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if matches!(lower.as_str(), "q" | "quit" | "exit") {
            return Ok(Action::Quit);
        }
        if let Ok(n) = trimmed.parse::<usize>() {
            if n >= 1 && n <= MENU.len() {
                return Ok(MENU[n - 1].0);
            }
        }
        eprintln!("  not a valid choice — try 1-{} (or 'q' to quit).", MENU.len());
    }
}

fn prompt_path(label: &str) -> Result<Option<PathBuf>> {
    eprintln!();
    eprintln!("  -> {label}");
    eprintln!("     (empty to cancel; quotes auto-stripped)");

    loop {
        let raw = read_line("  path: ")?;
        let cleaned = strip_path_quotes(raw.trim());
        if cleaned.is_empty() {
            return Ok(None);
        }
        let path = PathBuf::from(&cleaned);
        if path.exists() {
            return Ok(Some(path));
        }
        eprintln!("  not found: {cleaned} (try again or empty to cancel)");
    }
}

fn prompt_yes_no(label: &str, default_yes: bool) -> Result<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    loop {
        let raw = read_line(&format!("  {label} {hint} "))?;
        let trimmed = raw.trim().to_lowercase();
        if trimmed.is_empty() {
            return Ok(default_yes);
        }
        match trimmed.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => eprintln!("  answer 'y' or 'n'."),
        }
    }
}

fn read_line(prompt: &str) -> Result<String> {
    let mut stdout = io::stderr();
    write!(stdout, "{prompt}")?;
    stdout.flush()?;
    let mut buf = String::new();
    io::stdin().lock().read_line(&mut buf)?;
    while buf.ends_with('\n') || buf.ends_with('\r') {
        buf.pop();
    }
    Ok(buf)
}

fn strip_path_quotes(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

fn baseline_cli(input: PathBuf) -> Cli {
    Cli {
        input,
        output: None,
        info_only: false,
        verbose: false,
        json: false,
        log_level: LogLevel::Normal,
        no_pause: true,
    }
}
```

- [ ] **Step 2: Wire entry-mode detection into `main.rs`**

Modify `crates/mapgeo2fbx-cli/src/main.rs` — replace the `fn main()` body and add the entry-mode
enum/detector, keeping `run()` and `print_info()` from Task 6 unchanged:

```rust
mod args;
mod banner;
mod batch;
mod interactive;
mod logging;
mod ui;

use std::path::{Path, PathBuf};

use anyhow::Result;
use args::{Cli, LogLevel};
use clap::Parser;
use colored::Colorize;
use mapgeo2fbx_core::info::FileInfo;

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let mode = detect_entry_mode(&raw);

    let result = match mode {
        EntryMode::Interactive => {
            logging::init(LogLevel::Normal, false);
            let r = interactive::run();
            if let Err(ref e) = r {
                eprintln!("{} {e:#}", "error:".bright_red().bold());
            }
            if r.is_err() {
                std::process::exit(1);
            }
            return;
        }
        EntryMode::DragDrop(path) => {
            logging::init(LogLevel::Normal, false);
            banner::print();
            let cli = Cli {
                input: path,
                output: None,
                info_only: false,
                verbose: false,
                json: false,
                log_level: LogLevel::Normal,
                no_pause: false,
            };
            run(&cli)
        }
        EntryMode::Flagged => {
            let cli = Cli::parse_from(&raw);
            logging::init(cli.log_level, cli.json);
            banner::print();
            let r = run(&cli);
            let no_pause = cli.no_pause || cli.json;
            if let Err(ref e) = r {
                eprintln!("{} {e:#}", "error:".bright_red().bold());
            }
            if !no_pause {
                pause();
            }
            if r.is_err() {
                std::process::exit(1);
            }
            return;
        }
    };

    if let Err(ref e) = result {
        eprintln!("{} {e:#}", "error:".bright_red().bold());
    }
    pause();
    if result.is_err() {
        std::process::exit(1);
    }
}

fn pause() {
    eprintln!();
    eprintln!("Press Enter to exit...");
    let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0u8]);
}

enum EntryMode {
    Interactive,
    DragDrop(PathBuf),
    Flagged,
}

/// Picks an entry mode from the raw argv, before clap runs — mirrors `hematite-cli`'s
/// `detect_entry_mode`. A single existing path with no flags covers both a dropped file and a
/// dropped folder; `run()` dispatches on `is_file()`/`is_dir()` from there.
fn detect_entry_mode(raw: &[String]) -> EntryMode {
    let user_args: Vec<&str> = raw.iter().skip(1).map(|s| s.as_str()).collect();

    if user_args.is_empty() {
        return EntryMode::Interactive;
    }

    if user_args.len() == 1 {
        let only = user_args[0];
        if !only.starts_with('-') && Path::new(only).exists() {
            return EntryMode::DragDrop(PathBuf::from(only));
        }
    }

    EntryMode::Flagged
}

pub fn run(cli: &Cli) -> Result<()> {
    if !cli.input.exists() {
        anyhow::bail!("input path does not exist: {}", cli.input.display());
    }

    if cli.input.is_file() {
        let info = if cli.info_only {
            batch::load_info(&cli.input)?
        } else {
            batch::convert_one_file(&cli.input, cli.output.as_deref())?
        };
        print_info(&info, cli.json, cli.verbose);
        Ok(())
    } else {
        if cli.output.is_some() {
            anyhow::bail!("--output is only valid when converting a single file, not a directory");
        }
        let mode = ui::Mode::from_json(cli.json);
        let file_count = walkdir::WalkDir::new(&cli.input)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("mapgeo"))
                    .unwrap_or(false)
            })
            .count() as u64;

        let reporter = ui::UiReporter::new(mode, file_count);
        let summary = batch::convert_folder(&cli.input, &reporter);
        reporter.finish();

        if cli.json {
            let payload = serde_json::json!({
                "converted": summary.converted,
                "failed": summary.failed.iter().map(|(p, e)| serde_json::json!({
                    "path": p.display().to_string(),
                    "error": e,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
        } else {
            println!(
                "\nconverted {} file(s), {} failed",
                summary.converted,
                summary.failed.len()
            );
        }

        if !summary.failed.is_empty() {
            anyhow::bail!("{} file(s) failed to convert", summary.failed.len());
        }
        Ok(())
    }
}

fn print_info(info: &FileInfo, json: bool, verbose: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(info).expect("FileInfo serializes"));
        return;
    }
    if verbose {
        println!("{info}");
    } else {
        println!(
            "mapgeo version: {} | models: {} | vertices: {} | triangles: {} | materials: {} | size: {} bytes",
            info.version,
            info.model_count,
            info.total_vertices,
            info.total_triangles,
            info.unique_material_count,
            info.file_size_bytes
        );
    }
}
```

Note: `Cli` and `LogLevel` need `Clone`/no special derives beyond what clap already requires;
`interactive.rs`'s `baseline_cli` constructs a `Cli` directly, so no additional derive is needed
since it's built with named fields, not cloned from an existing instance.

- [ ] **Step 3: Write the failing test for flagged-mode still working with explicit args**

Append to `crates/mapgeo2fbx-cli/tests/cli_tests.rs`:

```rust
#[test]
fn flagged_mode_with_output_override_works() {
    let dir = tempdir().expect("tempdir");
    let input_path = dir.path().join("test.mapgeo");
    fs::write(&input_path, minimal_v17_bytes()).expect("write fixture");
    let output_path = dir.path().join("custom_name.fbx");

    Command::cargo_bin("mapgeo2fbx")
        .expect("binary exists")
        .arg(&input_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--no-pause")
        .arg("--json")
        .assert()
        .success();

    assert!(output_path.exists());
    assert!(!dir.path().join("test.fbx").exists());
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-cli flagged_mode_with_output_override_works`
Expected: FAIL to compile if `main.rs`'s restructuring introduced any signature mismatch (e.g.
`run` needing to be `pub` for `interactive.rs`'s `crate::run` call to resolve) — fix compile
errors first.

- [ ] **Step 5: Run the full CLI test suite to verify everything passes**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-cli`
Expected: PASS — all five integration tests succeed (`converts_minimal_mapgeo_to_fbx`,
`info_only_does_not_write_fbx`, `converts_folder_recursively`,
`flagged_mode_with_output_override_works`, plus entry-mode is exercised implicitly since these
all pass explicit flags/args which route through `EntryMode::Flagged`).

- [ ] **Step 6: Manually verify double-click and drag-drop behavior**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo build --release -p mapgeo2fbx-cli`
Then in Windows Explorer, double-click
`E:\RitoShark\mapgeo-converter\target\release\mapgeo2fbx.exe` — expect the banner + numbered
menu to appear in a console window, and picking option 1 to prompt for a path.
Then drag a `.mapgeo` file (e.g. one from
`E:\RitoShark\RitoShark - Crate\RitoShark-Crates\Sample-Files\bloom.mapgeo`) onto the same exe —
expect it to convert in place, printing the info summary, and to leave a `bloom.fbx` next to the
source file, pausing with "Press Enter to exit..." before closing.

- [ ] **Step 7: Commit**

```bash
git add crates/mapgeo2fbx-cli
git commit -m "Add double-click and drag-drop entry modes"
```

---

## Task 8: Real sample file smoke test

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\Sample-Files\` (copy one small sample)
- Test: `E:\RitoShark\mapgeo-converter\crates\mapgeo2fbx-core\tests\real_file_smoke.rs`

**Interfaces:**
- Consumes: `mapgeo2fbx_core::decode::decode_geometry`, `mapgeo2fbx_core::fbx::write_fbx`, `mapgeo2fbx_core::info::summarize` (Tasks 2–4); `ritoshark::mapgeo::MapGeometry`, `ritoshark::io::Parse` (external).
- Produces: nothing new — this is a pure verification task confirming the whole core pipeline works against a real, non-synthetic `.mapgeo` file, following `RitoShark-Crates`' own convention of skipping cleanly when the sample file isn't present.

- [ ] **Step 1: Copy a small real sample file into the new repo**

Run:
```
mkdir "E:\RitoShark\mapgeo-converter\Sample-Files"
copy "E:\RitoShark\RitoShark - Crate\RitoShark-Crates\Sample-Files\bloom.mapgeo" "E:\RitoShark\mapgeo-converter\Sample-Files\bloom.mapgeo"
```
(Use PowerShell `Copy-Item` if `copy` isn't available in the active shell.)

- [ ] **Step 2: Write the smoke test**

Create `crates/mapgeo2fbx-core/tests/real_file_smoke.rs`:

```rust
use std::path::Path;

use mapgeo2fbx_core::decode::decode_geometry;
use mapgeo2fbx_core::fbx::write_fbx;
use mapgeo2fbx_core::info::summarize;
use ritoshark::io::Parse;
use ritoshark::mapgeo::MapGeometry;

/// Exercises the full decode -> summarize -> write_fbx pipeline against a real map file.
/// Skips cleanly (rather than failing) if the sample isn't present, matching the convention
/// `RitoShark-Crates` itself uses for real-file tests that depend on non-committed game data.
#[test]
fn converts_real_sample_file() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sample-Files/bloom.mapgeo");
    if !path.exists() {
        eprintln!("skipping: sample file not present at {}", path.display());
        return;
    }

    let bytes = std::fs::read(&path).expect("read sample file");
    let geo = MapGeometry::from_bytes(&bytes).expect("parse real mapgeo file");
    let meshes = decode_geometry(&geo).expect("decode real mapgeo file");

    assert!(!meshes.is_empty(), "expected at least one decoded mesh");
    for mesh in &meshes {
        assert!(!mesh.vertices.is_empty(), "mesh {} has no vertices", mesh.name);
    }

    let info = summarize(&geo, &meshes, bytes.len() as u64);
    assert_eq!(info.model_count, meshes.len());
    assert!(info.total_vertices > 0);

    let mut buf = Vec::new();
    write_fbx(&mut buf, &meshes).expect("write real mapgeo file to fbx");
    let text = String::from_utf8(buf).expect("fbx output must be valid utf8");
    assert!(text.contains("FBXHeaderExtension"));
    assert!(text.contains("Objects:"));
    assert!(text.contains("Connections:"));
}
```

- [ ] **Step 3: Run the test**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo test -p mapgeo2fbx-core --test real_file_smoke -- --nocapture`
Expected: PASS. If it fails with `UnsupportedVertexFormat`, that means `bloom.mapgeo` uses a
packed vertex format (e.g. `XyzPacked161616`) that Task 2's `decode.rs` doesn't handle yet — in
that case, extend `read_vec3`/`read_vec2` in `decode.rs` to support the additional
`ElementFormat` variants actually present (check which ones by printing
`description.elements` for the failing model before writing the decode logic, rather than
guessing).

- [ ] **Step 4: Open the generated FBX in Blender to visually confirm**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo run --release -p mapgeo2fbx-cli -- Sample-Files/bloom.mapgeo --no-pause`
Then open `Sample-Files/bloom.fbx` in Blender (File > Import > FBX) and confirm geometry appears
with reasonable proportions and per-submesh material slots assigned (materials will render as
flat gray/lambert since no textures are wired in, per the spec's scope).

- [ ] **Step 5: Commit**

```bash
git add Sample-Files crates/mapgeo2fbx-core/tests/real_file_smoke.rs
git commit -m "Add real-file smoke test against a sample mapgeo"
```

---

## Task 9: README, DEVELOPER.md, LICENSE

**Files:**
- Create: `E:\RitoShark\mapgeo-converter\README.md`
- Create: `E:\RitoShark\mapgeo-converter\DEVELOPER.md`
- Create: `E:\RitoShark\mapgeo-converter\LICENSE-MIT`
- Create: `E:\RitoShark\mapgeo-converter\LICENSE-APACHE`

**Interfaces:**
- Consumes: nothing (documentation only).
- Produces: nothing consumed by other tasks — this is the final polish task.

- [ ] **Step 1: Write `README.md`**

```markdown
# mapgeo2fbx

Converts League of Legends `.mapgeo` map geometry files to ASCII `.fbx` for viewing/editing in
Blender, Maya, 3ds Max, and similar tools.

## Usage

**Double-click** `mapgeo2fbx.exe` for an interactive menu.

**Drag and drop** a `.mapgeo` file, or a folder containing `.mapgeo` files, onto the exe — it
converts in place, writing each `.fbx` next to its source file.

**Command line:**

```
mapgeo2fbx <input.mapgeo>              # convert one file
mapgeo2fbx <input.mapgeo> -o out.fbx   # convert to a specific output path
mapgeo2fbx <folder>                    # recursively convert every .mapgeo in a folder
mapgeo2fbx <input.mapgeo> --info-only  # inspect without converting
mapgeo2fbx <input.mapgeo> --verbose    # include a per-model breakdown
mapgeo2fbx <input.mapgeo> --json       # machine-readable output
```

Run `mapgeo2fbx --help` for the full flag list.

## Scope

- Geometry (positions, normals, primary UVs) and per-submesh material names — no textures are
  extracted or embedded, since `.mapgeo` only stores texture *paths*, not texture data.
- Static geometry only — `.mapgeo` has no skeleton/animation data (that's a different format
  used by character skins).
- ASCII FBX 7.4 output only.

## Building

Requires the Rust toolchain pinned in `rust-toolchain.toml` (installs automatically via
`rustup` if you have it).

```
cargo build --release -p mapgeo2fbx-cli
```

The binary is written to `target/release/mapgeo2fbx.exe`.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
```

- [ ] **Step 2: Write `DEVELOPER.md`**

```markdown
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
```

- [ ] **Step 3: Add license files**

Copy the license text from `RitoShark-Crates` (same MIT OR Apache-2.0 dual license convention):

Run:
```
copy "E:\RitoShark\RitoShark - Crate\RitoShark-Crates\LICENSE-MIT" "E:\RitoShark\mapgeo-converter\LICENSE-MIT"
copy "E:\RitoShark\RitoShark - Crate\RitoShark-Crates\LICENSE-APACHE" "E:\RitoShark\mapgeo-converter\LICENSE-APACHE"
```

- [ ] **Step 4: Verify the workspace still builds and all tests still pass after adding docs**

Run: `cd "E:\RitoShark\mapgeo-converter" && cargo build --workspace && cargo test --workspace`
Expected: builds and all tests pass (documentation changes don't affect compiled code).

- [ ] **Step 5: Commit**

```bash
git add README.md DEVELOPER.md LICENSE-MIT LICENSE-APACHE
git commit -m "Add README, developer notes, and license files"
```

---

## Self-Review Notes

- **Spec coverage:** every spec section maps to a task — source data/decoding (Task 2), FBX
  writer (Task 3), info summary (Task 4), CLI shell + single-file convert (Task 5), batch/folder
  (Task 6), entry-mode detection + interactive menu (Task 7), real-file validation (Task 8),
  project polish/docs (Task 9). Performance requirements (pure Rust, no FFI, rayon batch
  parallelism) are satisfied by the crate choices in Task 1 and the `par_iter` in Task 6.
- **Type consistency:** `DecodedMesh`/`DecodedSubmesh`/`DecodedVertex` (Task 2) are the single
  shared vocabulary used unchanged by `fbx.rs` (Task 3) and `info.rs` (Task 4). `FileInfo`/
  `ModelInfo` (Task 4) are the single vocabulary used unchanged by `main.rs`'s `print_info` (Task
  5) and the JSON test assertions (Tasks 5–7). `Cli`/`LogLevel` (Task 5) are constructed
  identically in `main.rs`'s flagged/drag-drop paths and `interactive.rs`'s `baseline_cli` (Task
  7) — same field set throughout.
- **No placeholders:** all code blocks are complete, runnable Rust: no TODOs, no "similar to
  above" references — the `Task 6`→`Task 7` `main.rs` rewrite reproduces `run()`/`print_info()`
  verbatim rather than cross-referencing the earlier version, since a task implementer sees only
  their own task.
