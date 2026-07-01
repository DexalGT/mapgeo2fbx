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
