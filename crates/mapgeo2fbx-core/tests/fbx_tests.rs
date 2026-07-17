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

#[test]
fn writes_expected_ascii_for_single_triangle() {
    let meshes = vec![quad_mesh()];
    let mut buf = Vec::new();
    write_fbx(&mut buf, &meshes).expect("write should succeed");
    let text = String::from_utf8(buf).expect("output must be valid utf8");

    assert!(text.contains(r#"FBXHeaderExtension:  {"#));
    assert!(text.contains(r#"Creator: "mapgeo2fbx""#));
    // ASCII FBX object names use the literal "Class::name" form (class word, "::", user name).
    // The raw \x00\x01 separator is the *binary* FBX internal encoding; writing those control
    // bytes into an ASCII file makes Maya's strict SDK parser fail to resolve the object, so the
    // mesh never binds to its transform (it shows an empty node). Blender tolerates either form.
    assert!(text.contains("Model: 1000000, \"Model::MapGeo_Instance_0\", \"Mesh\""));
    assert!(text.contains("Geometry: 1000002, \"Geometry::MapGeo_Instance_0\", \"Mesh\""));
    // No raw NUL/SOH separator bytes may appear anywhere in an ASCII FBX.
    assert!(!text.contains('\u{0}'));
    assert!(!text.contains('\u{1}'));
    assert!(text.contains("Vertices: *9 {"));
    assert!(text.contains("a: 0,0,0,1,0,0,1,1,0"));
    assert!(text.contains("PolygonVertexIndex: *3 {"));
    assert!(text.contains("a: 0,1,-3"));
    assert!(text.contains("Material: 1000003, \"Material::Materials/Grass\", \"\""));

    // Maya binds geometry to a transform via a mesh NodeAttribute, not the Geometry connection
    // alone. Each model gets its own NodeAttribute (subclass "Mesh") connected to it.
    assert!(text.contains("NodeAttribute: 1000001, \"NodeAttribute::\", \"Mesh\""));

    // Geometry -> Model, NodeAttribute -> Model, Model -> root, Material -> Model.
    assert!(text.contains(r#"C: "OO",1000002,1000000"#));
    assert!(text.contains(r#"C: "OO",1000001,1000000"#));
    assert!(text.contains(r#"C: "OO",1000000,0"#));
    assert!(text.contains(r#"C: "OO",1000003,1000000"#));

    // Maya validates a complete header and Definitions; GlobalSettings must be declared and present.
    assert!(text.contains("CreationTimeStamp:  {"));
    assert!(text.contains(r#"ObjectType: "GlobalSettings" {"#));
    assert!(text.contains(r#"ObjectType: "NodeAttribute" {"#));
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
