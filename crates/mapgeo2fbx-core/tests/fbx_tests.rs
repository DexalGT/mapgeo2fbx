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

    // ASCII objects use the "Class::name" form.
    assert!(text.contains("Model: 1000000, \"Model::MapGeo_Instance_0\", \"Mesh\""));
    assert!(text.contains("Geometry: 1000001, \"Geometry::MapGeo_Instance_0\", \"Mesh\""));
    assert!(text.contains("Material: 1000002, \"Material::Materials/Grass\", \"\""));

    // The critical property: without DefaultAttributeIndex on the Model, Maya imports the
    // transform but silently drops the mesh (verified against Maya 2023).
    assert!(text.contains(r#"P: "DefaultAttributeIndex", "int", "Integer", "",0"#));

    // No NodeAttribute — Geometry binds directly to its Model.
    assert!(!text.contains("NodeAttribute"));
    assert!(!text.contains('\u{0}'));
    assert!(!text.contains('\u{1}'));

    // Geometry data.
    assert!(text.contains("Vertices: *9 {"));
    assert!(text.contains("a: 0,0,0,1,0,0,1,1,0"));
    assert!(text.contains("PolygonVertexIndex: *3 {"));
    assert!(text.contains("a: 0,1,-3"));

    // Connections: Geometry -> Model, Model -> root, Material -> Model.
    assert!(text.contains(r#"C: "OO",1000001,1000000"#));
    assert!(text.contains(r#"C: "OO",1000000,0"#));
    assert!(text.contains(r#"C: "OO",1000002,1000000"#));
}

#[test]
fn wraps_large_arrays_across_multiple_short_lines() {
    // A mesh with enough vertices that the vertex array exceeds one wrapped line. Maya's ASCII
    // importer chokes on multi-megabyte single lines, so no `a:` line may run unbounded.
    let mut vertices = Vec::new();
    let mut tris = Vec::new();
    for i in 0..300u32 {
        vertices.push(DecodedVertex {
            position: Vec3::new(i as f32, 0.0, 0.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            uv0: Vec2::new(0.0, 0.0),
        });
        if i % 3 == 2 {
            tris.push([i - 2, i - 1, i]);
        }
    }
    let mesh = DecodedMesh {
        name: "Big".to_string(),
        vertices,
        submeshes: vec![DecodedSubmesh {
            name: "M".to_string(),
            triangle_indices: tris,
        }],
    };

    let mut buf = Vec::new();
    write_fbx(&mut buf, &[mesh]).expect("write should succeed");
    let text = String::from_utf8(buf).expect("output must be valid utf8");

    // 900 vertex floats wrapped at 64 per line means many lines, none extremely long.
    let longest = text.lines().map(|l| l.len()).max().unwrap_or(0);
    assert!(
        longest < 2000,
        "no line should be huge; longest was {longest}"
    );
    // The array count marker still reflects the full length.
    assert!(text.contains("Vertices: *900 {"));
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
