use std::collections::HashMap;
use std::io::Write;

use crate::decode::DecodedMesh;
use crate::error::{Error, Result};

/// Writes a **binary** FBX 7.4 scene containing one Model + Geometry pair per input mesh, with
/// per-submesh material assignment via a `ByPolygon`/`IndexToDirect` `LayerElementMaterial`.
/// Material nodes are deduplicated by name across the whole scene.
///
/// The output targets Autodesk Maya. An earlier ASCII writer produced files that Blender read but
/// Maya imported as empty transform nodes: Maya's ASCII importer drops meshes whose vertex arrays
/// are written as a single multi-megabyte line. Binary FBX stores every array as a length-prefixed
/// typed block, so there is no line-length fragility — this is the exact encoding Maya's own
/// exporter uses. The Geometry connects directly to its Model (no NodeAttribute); a decoded Maya
/// export confirms mesh models bind their geometry that way.
pub fn write_fbx<W: Write>(writer: &mut W, meshes: &[DecodedMesh]) -> Result<()> {
    // Approach: serialize the whole document into an in-memory buffer so node end-offsets can be
    // back-patched once each node's children are written, then flush the buffer to `writer`. This
    // keeps the public signature `W: Write` (no Seek bound on callers).
    let mut b = Buf::new();

    let mut next_id: i64 = 1_000_000;
    let mut alloc_id = move || {
        let id = next_id;
        next_id += 1;
        id
    };

    // Model/Geometry ids per mesh first, then material ids, so ids stay stable and low.
    let mut model_geometry_ids = Vec::with_capacity(meshes.len());
    for _mesh in meshes {
        let model_id = alloc_id();
        let geometry_id = alloc_id();
        model_geometry_ids.push((model_id, geometry_id));
    }
    let mut material_ids: HashMap<String, i64> = HashMap::new();
    for mesh in meshes {
        for submesh in &mesh.submeshes {
            material_ids
                .entry(submesh.name.clone())
                .or_insert_with(&mut alloc_id);
        }
    }

    write_document(&mut b, meshes, &model_geometry_ids, &material_ids)?;

    writer.write_all(&b.into_bytes())?;
    Ok(())
}

fn write_document(
    b: &mut Buf,
    meshes: &[DecodedMesh],
    model_geometry_ids: &[(i64, i64)],
    material_ids: &HashMap<String, i64>,
) -> Result<()> {
    // Binary FBX header: 21-byte magic, two bytes {0x1A, 0x00}, then the u32 version.
    b.raw(b"Kaydara FBX Binary  \x00");
    b.raw(&[0x1a, 0x00]);
    b.u32(7400);

    write_header_extension(b);
    top_level(b, "FileId", |b| {
        // A fixed 16-byte id; content is not validated by importers.
        b.prop_raw_bytes(&[0u8; 16]);
    });
    top_level(b, "CreationTime", |b| {
        b.prop_str("1970-01-01 00:00:00:000");
    });
    top_level(b, "Creator", |b| {
        b.prop_str("mapgeo2fbx");
    });

    write_global_settings(b);
    write_documents(b);
    top_level(b, "References", |_b| {});
    write_definitions(b, meshes.len(), material_ids.len());
    write_objects(b, meshes, model_geometry_ids, material_ids)?;
    write_connections(b, meshes, model_geometry_ids, material_ids);

    // A top-level null record closes the node list, then the footer.
    b.null_record();
    write_footer(b);
    Ok(())
}

fn write_header_extension(b: &mut Buf) {
    node(b, "FBXHeaderExtension", |_| {}, |b| {
        leaf_i32(b, "FBXHeaderVersion", 1003);
        leaf_i32(b, "FBXVersion", 7400);
        node(b, "CreationTimeStamp", |_| {}, |b| {
            leaf_i32(b, "Version", 1000);
            leaf_i32(b, "Year", 1970);
            leaf_i32(b, "Month", 1);
            leaf_i32(b, "Day", 1);
            leaf_i32(b, "Hour", 0);
            leaf_i32(b, "Minute", 0);
            leaf_i32(b, "Second", 0);
            leaf_i32(b, "Millisecond", 0);
        });
        node(b, "Creator", |b| b.prop_str("mapgeo2fbx"), |_| {});
    });
}

fn write_global_settings(b: &mut Buf) {
    node(b, "GlobalSettings", |_| {}, |b| {
        leaf_i32(b, "Version", 1000);
        node(b, "Properties70", |_| {}, |b| {
            prop70_int(b, "UpAxis", 1);
            prop70_int(b, "UpAxisSign", 1);
            prop70_int(b, "FrontAxis", 2);
            prop70_int(b, "FrontAxisSign", 1);
            prop70_int(b, "CoordAxis", 0);
            prop70_int(b, "CoordAxisSign", 1);
            prop70_double(b, "UnitScaleFactor", 1.0);
        });
    });
}

fn write_documents(b: &mut Buf) {
    node(b, "Documents", |_| {}, |b| {
        leaf_i32(b, "Count", 1);
        node(
            b,
            "Document",
            |b| {
                b.prop_i64(1_000_000_000);
                b.prop_str("Scene");
                b.prop_str("Scene");
            },
            |b| {
                node(b, "RootNode", |b| b.prop_i64(0), |_| {});
            },
        );
    });
}

fn write_definitions(b: &mut Buf, model_count: usize, material_count: usize) {
    // One GlobalSettings + one Model and Geometry per mesh + one Material per unique name.
    let total = 1 + model_count * 2 + material_count;
    node(b, "Definitions", |_| {}, |b| {
        leaf_i32(b, "Version", 100);
        leaf_i32(b, "Count", total as i32);
        object_type(b, "GlobalSettings", 1);
        object_type(b, "Model", model_count as i32);
        object_type(b, "Geometry", model_count as i32);
        object_type(b, "Material", material_count as i32);
    });
}

fn object_type(b: &mut Buf, name: &str, count: i32) {
    node(b, "ObjectType", |b| b.prop_str(name), |b| {
        leaf_i32(b, "Count", count);
    });
}

fn write_objects(
    b: &mut Buf,
    meshes: &[DecodedMesh],
    model_geometry_ids: &[(i64, i64)],
    material_ids: &HashMap<String, i64>,
) -> Result<()> {
    // `node` closures cannot return a Result, so build the geometry payloads first (which is where
    // index-out-of-range is detected) and only then emit the tree.
    let mut geometries = Vec::with_capacity(meshes.len());
    for mesh in meshes {
        geometries.push(build_geometry(mesh)?);
    }

    node_result(b, "Objects", |_| {}, |b| {
        for ((mesh, (model_id, geometry_id)), geometry) in
            meshes.iter().zip(model_geometry_ids).zip(&geometries)
        {
            write_model(b, *model_id, &mesh.name);
            write_geometry(b, *geometry_id, &mesh.name, geometry);
        }
        let mut sorted_materials: Vec<(&String, &i64)> = material_ids.iter().collect();
        sorted_materials.sort_by_key(|(_, id)| **id);
        for (name, id) in sorted_materials {
            write_material(b, *id, name);
        }
        Ok::<(), Error>(())
    })
}

fn write_model(b: &mut Buf, model_id: i64, name: &str) {
    node(
        b,
        "Model",
        |b| {
            b.prop_i64(model_id);
            b.prop_name_class(name, "Model");
            b.prop_str("Mesh");
        },
        |b| {
            leaf_i32(b, "Version", 232);
            node(b, "Properties70", |_| {}, |b| {
                // World transform is baked into the vertices, so the node stays at identity.
                prop70_lcl(b, "Lcl Translation", 0.0, 0.0, 0.0);
                prop70_lcl(b, "Lcl Rotation", 0.0, 0.0, 0.0);
                prop70_lcl(b, "Lcl Scaling", 1.0, 1.0, 1.0);
            });
            node(b, "Shading", |b| b.prop_bool(true), |_| {});
            node(b, "Culling", |b| b.prop_str("CullingOff"), |_| {});
        },
    );
}

/// Precomputed geometry arrays for one mesh, in FBX order.
struct GeometryData {
    positions: Vec<f64>,
    polygon_vertex_index: Vec<i32>,
    normals: Vec<f64>,
    uvs: Vec<f64>,
    uv_index: Vec<i32>,
    material_per_polygon: Vec<i32>,
}

fn build_geometry(mesh: &DecodedMesh) -> Result<GeometryData> {
    let mut name_to_local_index: HashMap<&str, i32> = HashMap::new();
    let mut next_local = 0i32;
    for submesh in &mesh.submeshes {
        name_to_local_index.entry(submesh.name.as_str()).or_insert_with(|| {
            let i = next_local;
            next_local += 1;
            i
        });
    }

    let positions: Vec<f64> = mesh
        .vertices
        .iter()
        .flat_map(|v| {
            [
                v.position.x as f64,
                v.position.y as f64,
                v.position.z as f64,
            ]
        })
        .collect();

    let mut polygon_vertex_index: Vec<i32> = Vec::new();
    let mut material_per_polygon: Vec<i32> = Vec::new();
    let mut normals: Vec<f64> = Vec::new();
    let mut uv_index: Vec<i32> = Vec::new();
    for submesh in &mesh.submeshes {
        let local = name_to_local_index[submesh.name.as_str()];
        for tri in &submesh.triangle_indices {
            // FBX marks the last index of a polygon by bitwise-negation (~i == -i - 1).
            polygon_vertex_index.push(tri[0] as i32);
            polygon_vertex_index.push(tri[1] as i32);
            polygon_vertex_index.push(!(tri[2] as i32));
            material_per_polygon.push(local);
            for &vi in tri {
                let v = mesh
                    .vertices
                    .get(vi as usize)
                    .ok_or_else(|| Error::VertexIndexOutOfRange {
                        mesh: mesh.name.clone(),
                        index: vi,
                        vertex_count: mesh.vertices.len(),
                    })?;
                normals.push(v.normal.x as f64);
                normals.push(v.normal.y as f64);
                normals.push(v.normal.z as f64);
                uv_index.push(vi as i32);
            }
        }
    }

    let uvs: Vec<f64> = mesh
        .vertices
        .iter()
        .flat_map(|v| [v.uv0.x as f64, v.uv0.y as f64])
        .collect();

    Ok(GeometryData {
        positions,
        polygon_vertex_index,
        normals,
        uvs,
        uv_index,
        material_per_polygon,
    })
}

fn write_geometry(b: &mut Buf, geometry_id: i64, name: &str, g: &GeometryData) {
    node(
        b,
        "Geometry",
        |b| {
            b.prop_i64(geometry_id);
            b.prop_name_class(name, "Geometry");
            b.prop_str("Mesh");
        },
        |b| {
            node(b, "Vertices", |b| b.prop_array_f64(&g.positions), |_| {});
            node(
                b,
                "PolygonVertexIndex",
                |b| b.prop_array_i32(&g.polygon_vertex_index),
                |_| {},
            );
            leaf_i32(b, "GeometryVersion", 124);

            // Normals: ByPolygonVertex/Direct, one triplet per (polygon, vertex-in-polygon).
            node(b, "LayerElementNormal", |b| b.prop_i32(0), |b| {
                leaf_i32(b, "Version", 101);
                node(b, "Name", |b| b.prop_str(""), |_| {});
                leaf_str(b, "MappingInformationType", "ByPolygonVertex");
                leaf_str(b, "ReferenceInformationType", "Direct");
                node(b, "Normals", |b| b.prop_array_f64(&g.normals), |_| {});
            });

            // UVs: Direct per-vertex array + IndexToDirect index list matching polygon order.
            node(b, "LayerElementUV", |b| b.prop_i32(0), |b| {
                leaf_i32(b, "Version", 101);
                node(b, "Name", |b| b.prop_str(""), |_| {});
                leaf_str(b, "MappingInformationType", "ByPolygonVertex");
                leaf_str(b, "ReferenceInformationType", "IndexToDirect");
                node(b, "UV", |b| b.prop_array_f64(&g.uvs), |_| {});
                node(b, "UVIndex", |b| b.prop_array_i32(&g.uv_index), |_| {});
            });

            // Materials: one index per triangle (ByPolygon/IndexToDirect).
            node(b, "LayerElementMaterial", |b| b.prop_i32(0), |b| {
                leaf_i32(b, "Version", 101);
                node(b, "Name", |b| b.prop_str(""), |_| {});
                leaf_str(b, "MappingInformationType", "ByPolygon");
                leaf_str(b, "ReferenceInformationType", "IndexToDirect");
                node(
                    b,
                    "Materials",
                    |b| b.prop_array_i32(&g.material_per_polygon),
                    |_| {},
                );
            });

            node(b, "Layer", |b| b.prop_i32(0), |b| {
                leaf_i32(b, "Version", 100);
                layer_element(b, "LayerElementNormal");
                layer_element(b, "LayerElementMaterial");
                layer_element(b, "LayerElementUV");
            });
        },
    );
}

fn layer_element(b: &mut Buf, type_name: &str) {
    node(b, "LayerElement", |_| {}, |b| {
        leaf_str(b, "Type", type_name);
        leaf_i32(b, "TypedIndex", 0);
    });
}

fn write_material(b: &mut Buf, material_id: i64, name: &str) {
    node(
        b,
        "Material",
        |b| {
            b.prop_i64(material_id);
            b.prop_name_class(name, "Material");
            b.prop_str("");
        },
        |b| {
            leaf_i32(b, "Version", 102);
            node(b, "ShadingModel", |b| b.prop_str("Lambert"), |_| {});
            leaf_i32(b, "MultiLayer", 0);
            node(b, "Properties70", |_| {}, |b| {
                prop70_color(b, "DiffuseColor", 0.8, 0.8, 0.8);
            });
        },
    );
}

fn write_connections(
    b: &mut Buf,
    meshes: &[DecodedMesh],
    model_geometry_ids: &[(i64, i64)],
    material_ids: &HashMap<String, i64>,
) {
    node(b, "Connections", |_| {}, |b| {
        for (mesh, (model_id, geometry_id)) in meshes.iter().zip(model_geometry_ids) {
            connect_oo(b, *geometry_id, *model_id);
            connect_oo(b, *model_id, 0);
            let mut seen = std::collections::HashSet::new();
            for submesh in &mesh.submeshes {
                if seen.insert(&submesh.name) {
                    connect_oo(b, material_ids[&submesh.name], *model_id);
                }
            }
        }
    });
}

fn connect_oo(b: &mut Buf, src: i64, dst: i64) {
    node(
        b,
        "C",
        |b| {
            b.prop_str("OO");
            b.prop_i64(src);
            b.prop_i64(dst);
        },
        |_| {},
    );
}

fn write_footer(b: &mut Buf) {
    // The FBX binary footer: a fixed magic, zero padding to a 16-byte boundary, then the version
    // and a second fixed magic. These byte sequences are constant across FBX SDK exports.
    const FOOTER_MAGIC1: [u8; 16] = [
        0xfa, 0xbc, 0xab, 0x09, 0xd0, 0xc8, 0xd4, 0x66, 0xb1, 0x76, 0xfb, 0x83, 0x1c, 0xf7, 0x26,
        0x7e,
    ];
    const FOOTER_MAGIC2: [u8; 16] = [
        0xf8, 0x5a, 0x8c, 0x6a, 0xde, 0xf5, 0xd9, 0x7e, 0xec, 0xe9, 0x0c, 0xe3, 0x75, 0x8f, 0x29,
        0x0b,
    ];
    b.raw(&FOOTER_MAGIC1);
    // Pad with zeros so the offset after the magic is a multiple of 16.
    while !b.data.len().is_multiple_of(16) {
        b.raw(&[0]);
    }
    // 4 bytes of zero padding, then the version, then 120 zero bytes, then the closing magic.
    b.raw(&[0u8; 4]);
    b.u32(7400);
    b.raw(&[0u8; 120]);
    b.raw(&FOOTER_MAGIC2);
}

// ---------------------------------------------------------------------------
// Binary FBX node/property writer with back-patched end offsets.
// ---------------------------------------------------------------------------

struct Buf {
    data: Vec<u8>,
    /// Count of properties written since the last snapshot; used to fill in a node's
    /// NumProperties field. Snapshotted and restored around each node's property closure.
    prop_count: usize,
}

impl Buf {
    fn new() -> Self {
        Buf {
            data: Vec::new(),
            prop_count: 0,
        }
    }
    fn into_bytes(self) -> Vec<u8> {
        self.data
    }
    fn raw(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }
    fn u32(&mut self, v: u32) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    fn patch_u32(&mut self, at: usize, v: u32) {
        self.data[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    // --- properties (each bumps prop_count) ---
    fn prop_i32(&mut self, v: i32) {
        self.prop_count += 1;
        self.data.push(b'I');
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    fn prop_i64(&mut self, v: i64) {
        self.prop_count += 1;
        self.data.push(b'L');
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    fn prop_f64(&mut self, v: f64) {
        self.prop_count += 1;
        self.data.push(b'D');
        self.data.extend_from_slice(&v.to_le_bytes());
    }
    fn prop_bool(&mut self, v: bool) {
        self.prop_count += 1;
        self.data.push(b'C');
        self.data.push(if v { 1 } else { 0 });
    }
    fn prop_str(&mut self, s: &str) {
        self.prop_count += 1;
        self.data.push(b'S');
        self.u32(s.len() as u32);
        self.data.extend_from_slice(s.as_bytes());
    }
    fn prop_raw_bytes(&mut self, bytes: &[u8]) {
        self.prop_count += 1;
        self.data.push(b'R');
        self.u32(bytes.len() as u32);
        self.data.extend_from_slice(bytes);
    }
    /// Object name/class in the binary internal form `name\x00\x01Class`.
    fn prop_name_class(&mut self, name: &str, class: &str) {
        self.prop_count += 1;
        self.data.push(b'S');
        let len = name.len() + 2 + class.len();
        self.u32(len as u32);
        self.data.extend_from_slice(name.as_bytes());
        self.data.push(0x00);
        self.data.push(0x01);
        self.data.extend_from_slice(class.as_bytes());
    }
    fn prop_array_f64(&mut self, values: &[f64]) {
        self.prop_count += 1;
        self.data.push(b'd');
        self.u32(values.len() as u32);
        self.u32(0); // encoding: uncompressed
        self.u32((values.len() * 8) as u32);
        for &v in values {
            self.data.extend_from_slice(&v.to_le_bytes());
        }
    }
    fn prop_array_i32(&mut self, values: &[i32]) {
        self.prop_count += 1;
        self.data.push(b'i');
        self.u32(values.len() as u32);
        self.u32(0); // encoding: uncompressed
        self.u32((values.len() * 4) as u32);
        for &v in values {
            self.data.extend_from_slice(&v.to_le_bytes());
        }
    }

    fn null_record(&mut self) {
        // 13 zero bytes: EndOffset, NumProperties, PropertyListLen, NameLen.
        self.raw(&[0u8; 13]);
    }
}

/// Emits a node: a header with back-patched EndOffset/NumProperties/PropertyListLen, then the
/// property list (via `props`), then nested child nodes (via `children`). A node with children
/// gets a trailing 13-byte null record, per the binary FBX spec.
fn node(
    b: &mut Buf,
    name: &str,
    props: impl FnOnce(&mut Buf),
    children: impl FnOnce(&mut Buf),
) {
    node_result::<_, _, ()>(b, name, props, |b| {
        children(b);
        Ok(())
    })
    .expect("infallible node")
}

fn node_result<P, C, E>(b: &mut Buf, name: &str, props: P, children: C) -> std::result::Result<(), E>
where
    P: FnOnce(&mut Buf),
    C: FnOnce(&mut Buf) -> std::result::Result<(), E>,
{
    let header_at = b.data.len();
    b.u32(0); // EndOffset (patched)
    b.u32(0); // NumProperties (patched)
    b.u32(0); // PropertyListLen (patched)
    b.data.push(name.len() as u8);
    b.data.extend_from_slice(name.as_bytes());

    // Count only the properties written by *this* node's prop closure, not any nested nodes'.
    let saved_count = b.prop_count;
    b.prop_count = 0;
    let props_start = b.data.len();
    props(b);
    let num_props = b.prop_count;
    let prop_list_len = b.data.len() - props_start;
    b.prop_count = saved_count;

    let has_children_start = b.data.len();
    children(b)?;
    let wrote_children = b.data.len() > has_children_start;

    if wrote_children {
        b.null_record();
    }

    let end = b.data.len();
    b.patch_u32(header_at, end as u32);
    b.patch_u32(header_at + 4, num_props as u32);
    b.patch_u32(header_at + 8, prop_list_len as u32);
    Ok(())
}

// --- small helpers for common leaf nodes ---

fn leaf_i32(b: &mut Buf, name: &str, v: i32) {
    node(b, name, |b| b.prop_i32(v), |_| {});
}
fn leaf_str(b: &mut Buf, name: &str, v: &str) {
    node(b, name, |b| b.prop_str(v), |_| {});
}

fn top_level(b: &mut Buf, name: &str, props: impl FnOnce(&mut Buf)) {
    node(b, name, props, |_| {});
}

fn prop70_int(b: &mut Buf, name: &str, v: i32) {
    node(
        b,
        "P",
        |b| {
            b.prop_str(name);
            b.prop_str("int");
            b.prop_str("Integer");
            b.prop_str("");
            b.prop_i32(v);
        },
        |_| {},
    );
}
fn prop70_double(b: &mut Buf, name: &str, v: f64) {
    node(
        b,
        "P",
        |b| {
            b.prop_str(name);
            b.prop_str("double");
            b.prop_str("Number");
            b.prop_str("");
            b.prop_f64(v);
        },
        |_| {},
    );
}
fn prop70_lcl(b: &mut Buf, name: &str, x: f64, y: f64, z: f64) {
    node(
        b,
        "P",
        |b| {
            b.prop_str(name);
            b.prop_str(name);
            b.prop_str("");
            b.prop_str("A");
            b.prop_f64(x);
            b.prop_f64(y);
            b.prop_f64(z);
        },
        |_| {},
    );
}
fn prop70_color(b: &mut Buf, name: &str, r: f64, g: f64, bl: f64) {
    node(
        b,
        "P",
        |b| {
            b.prop_str(name);
            b.prop_str("Color");
            b.prop_str("");
            b.prop_str("A");
            b.prop_f64(r);
            b.prop_f64(g);
            b.prop_f64(bl);
        },
        |_| {},
    );
}
