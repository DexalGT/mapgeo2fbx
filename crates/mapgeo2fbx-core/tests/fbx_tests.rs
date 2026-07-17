use mapgeo2fbx_core::decode::{DecodedMesh, DecodedSubmesh, DecodedVertex};
use mapgeo2fbx_core::fbx::write_fbx;
use mapgeo2fbx_core::Error;
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

// ---------------------------------------------------------------------------
// Minimal binary-FBX reader, used only to verify the writer's output. It walks
// the node tree (32-bit offset variant, FBX 7400) and collects node names,
// string/int properties, and array lengths so tests can assert on structure
// without depending on an external FBX library.
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct FbxNode {
    name: String,
    props: Vec<FbxProp>,
    children: Vec<FbxNode>,
}

#[derive(Debug, Clone)]
enum FbxProp {
    I32(i32),
    I64(i64),
    F64(f64),
    Bool(bool),
    Str(String),
    ArrayF64(Vec<f64>),
    ArrayI32(Vec<i32>),
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> u8 {
        let v = self.data[self.pos];
        self.pos += 1;
        v
    }
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        v
    }
    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }
    fn i64(&mut self) -> i64 {
        let v = i64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        v
    }
    fn f64(&mut self) -> f64 {
        let v = f64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        v
    }
    fn bytes(&mut self, n: usize) -> &'a [u8] {
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        s
    }

    fn read_prop(&mut self) -> FbxProp {
        let t = self.u8() as char;
        match t {
            'I' => FbxProp::I32(self.i32()),
            'L' => FbxProp::I64(self.i64()),
            'D' => FbxProp::F64(self.f64()),
            'C' => FbxProp::Bool(self.u8() != 0),
            'S' | 'R' => {
                let len = self.u32() as usize;
                let raw = self.bytes(len);
                // The binary name/class separator is \x00\x01; normalize it to "::" so tests can
                // assert on the readable "name::Class" form.
                FbxProp::Str(String::from_utf8_lossy(raw).replace("\u{0}\u{1}", "::"))
            }
            'd' | 'i' | 'l' | 'f' | 'b' => {
                let array_len = self.u32() as usize;
                let encoding = self.u32();
                let comp_len = self.u32() as usize;
                assert_eq!(encoding, 0, "test reader only handles uncompressed arrays");
                let raw = self.bytes(comp_len);
                match t {
                    'd' => {
                        let mut v = Vec::with_capacity(array_len);
                        for i in 0..array_len {
                            v.push(f64::from_le_bytes(
                                raw[i * 8..i * 8 + 8].try_into().unwrap(),
                            ));
                        }
                        FbxProp::ArrayF64(v)
                    }
                    'i' => {
                        let mut v = Vec::with_capacity(array_len);
                        for i in 0..array_len {
                            v.push(i32::from_le_bytes(
                                raw[i * 4..i * 4 + 4].try_into().unwrap(),
                            ));
                        }
                        FbxProp::ArrayI32(v)
                    }
                    other => panic!("unexpected array type {other}"),
                }
            }
            other => panic!("unexpected property type {other:?}"),
        }
    }

    fn read_node(&mut self) -> Option<FbxNode> {
        let end_offset = self.u32() as usize;
        let num_props = self.u32() as usize;
        let _prop_list_len = self.u32();
        let name_len = self.u8() as usize;
        if end_offset == 0 {
            return None; // 13-byte null record
        }
        let name = String::from_utf8_lossy(self.bytes(name_len)).into_owned();
        let mut props = Vec::with_capacity(num_props);
        for _ in 0..num_props {
            props.push(self.read_prop());
        }
        let mut children = Vec::new();
        while self.pos < end_offset {
            match self.read_node() {
                Some(c) => children.push(c),
                None => break,
            }
        }
        self.pos = end_offset;
        Some(FbxNode {
            name,
            props,
            children,
        })
    }
}

fn parse_fbx(data: &[u8]) -> Vec<FbxNode> {
    assert!(
        data.starts_with(b"Kaydara FBX Binary  "),
        "output must be a binary FBX"
    );
    let mut r = Reader {
        data,
        pos: 27, // 21-byte magic + 2 unknown + 4-byte version
    };
    let mut roots = Vec::new();
    // Stop before the footer; top-level ends at the first null record.
    while r.pos < data.len() {
        match r.read_node() {
            Some(n) => roots.push(n),
            None => break,
        }
    }
    roots
}

fn find<'a>(nodes: &'a [FbxNode], name: &str) -> Option<&'a FbxNode> {
    nodes.iter().find(|n| n.name == name)
}

fn prop_str(p: &FbxProp) -> &str {
    match p {
        FbxProp::Str(s) => s.as_str(),
        _ => panic!("expected string prop, got {p:?}"),
    }
}

#[test]
fn writes_binary_fbx_with_bound_mesh_geometry() {
    let meshes = vec![quad_mesh()];
    let mut buf = Vec::new();
    write_fbx(&mut buf, &meshes).expect("write should succeed");

    // Must be a binary FBX (fixes Maya dropping the mesh from huge ASCII arrays).
    assert!(buf.starts_with(b"Kaydara FBX Binary  "));
    let version = u32::from_le_bytes(buf[23..27].try_into().unwrap());
    assert_eq!(version, 7400);

    let roots = parse_fbx(&buf);

    // Header/definitions Maya validates.
    let header = find(&roots, "FBXHeaderExtension").expect("FBXHeaderExtension present");
    assert!(find(&header.children, "CreationTimeStamp").is_some());
    let defs = find(&roots, "Definitions").expect("Definitions present");
    let object_types: Vec<&str> = defs
        .children
        .iter()
        .filter(|c| c.name == "ObjectType")
        .map(|c| prop_str(&c.props[0]))
        .collect();
    assert!(object_types.contains(&"Model"));
    assert!(object_types.contains(&"Geometry"));
    assert!(object_types.contains(&"Material"));
    // We no longer emit a per-mesh NodeAttribute; Maya binds Geometry to Model directly.
    assert!(!object_types.contains(&"NodeAttribute"));

    let objects = find(&roots, "Objects").expect("Objects present");
    let models: Vec<&FbxNode> = objects.children.iter().filter(|c| c.name == "Model").collect();
    let geoms: Vec<&FbxNode> = objects
        .children
        .iter()
        .filter(|c| c.name == "Geometry")
        .collect();
    assert_eq!(models.len(), 1);
    assert_eq!(geoms.len(), 1);
    assert!(objects.children.iter().all(|c| c.name != "NodeAttribute"));

    // Model/Geometry declared as meshes with the "name<sep>Class" internal form.
    let model = models[0];
    assert_eq!(prop_str(&model.props[1]), "MapGeo_Instance_0::Model");
    assert_eq!(prop_str(&model.props[2]), "Mesh");
    let geom = geoms[0];
    assert_eq!(prop_str(&geom.props[1]), "MapGeo_Instance_0::Geometry");
    assert_eq!(prop_str(&geom.props[2]), "Mesh");

    // Geometry holds the actual vertex data as a double array (9 floats = 3 verts).
    let verts = find(&geom.children, "Vertices").expect("Vertices present");
    match &verts.props[0] {
        FbxProp::ArrayF64(v) => {
            assert_eq!(v.len(), 9);
            assert_eq!(&v[0..3], &[0.0, 0.0, 0.0]);
            assert_eq!(&v[3..6], &[1.0, 0.0, 0.0]);
        }
        other => panic!("Vertices must be a double array, got {other:?}"),
    }
    let indices = find(&geom.children, "PolygonVertexIndex").expect("indices present");
    match &indices.props[0] {
        FbxProp::ArrayI32(v) => assert_eq!(v, &[0, 1, -3]), // last vertex of polygon is ~i
        other => panic!("PolygonVertexIndex must be an int array, got {other:?}"),
    }

    // Ids: get the Model and Geometry ids, then verify the connection graph binds them.
    let model_id = match &model.props[0] {
        FbxProp::I64(id) => *id,
        other => panic!("model id must be i64, got {other:?}"),
    };
    let geom_id = match &geom.props[0] {
        FbxProp::I64(id) => *id,
        other => panic!("geometry id must be i64, got {other:?}"),
    };

    let connections = find(&roots, "Connections").expect("Connections present");
    let oo: Vec<(i64, i64)> = connections
        .children
        .iter()
        .filter(|c| c.name == "C")
        .filter_map(|c| match (&c.props[0], &c.props[1], &c.props[2]) {
            (FbxProp::Str(kind), FbxProp::I64(a), FbxProp::I64(b)) if kind == "OO" => {
                Some((*a, *b))
            }
            _ => None,
        })
        .collect();
    // Geometry -> Model, and Model -> root (0). No NodeAttribute link.
    assert!(oo.contains(&(geom_id, model_id)));
    assert!(oo.contains(&(model_id, 0)));
}

#[test]
fn out_of_range_vertex_index_returns_error_not_panic() {
    let mut mesh = quad_mesh();
    // Corrupt a triangle index so it points past the 3-vertex mesh built by quad_mesh(). This
    // mirrors a corrupt submesh from a malformed .mapgeo file and must produce a proper Err
    // rather than panicking on the slice index.
    mesh.submeshes[0].triangle_indices[0] = [0, 1, 99];

    let meshes = vec![mesh];
    let mut buf = Vec::new();
    let err = write_fbx(&mut buf, &meshes).expect_err("out-of-range vertex index should error");
    match err {
        Error::VertexIndexOutOfRange {
            mesh,
            index,
            vertex_count,
        } => {
            assert_eq!(mesh, "MapGeo_Instance_0");
            assert_eq!(index, 99);
            assert_eq!(vertex_count, 3);
        }
        other => panic!("expected VertexIndexOutOfRange, got {other:?}"),
    }
}
