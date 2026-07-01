use mapgeo2fbx_core::decode::decode_geometry;
use mapgeo2fbx_core::Error;
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

#[test]
fn submesh_index_range_beyond_index_buffer_returns_error_not_panic() {
    let mut geo = one_triangle_geometry();
    // Corrupt the submesh's index range so it reaches past the 3-entry index buffer built by
    // one_triangle_geometry(). This mirrors a malformed/malicious .mapgeo file and must produce
    // a proper Err rather than panicking on the slice index.
    geo.models[0].submeshes[0].index_start = 0;
    geo.models[0].submeshes[0].index_count = 100;

    let err = decode_geometry(&geo).expect_err("out-of-range submesh index should error");
    match err {
        Error::SubmeshIndexOutOfRange {
            model,
            start,
            end,
            buffer_len,
        } => {
            assert_eq!(model, "MapGeo_Instance_0");
            assert_eq!(start, 0);
            assert_eq!(end, 100);
            assert_eq!(buffer_len, 3);
        }
        other => panic!("expected SubmeshIndexOutOfRange, got {other:?}"),
    }
}

#[test]
fn truncated_vertex_buffer_returns_error_not_panic() {
    let mut geo = one_triangle_geometry();
    // Truncate the vertex buffer so the last vertex's Texcoord0 element reads past the end of
    // the data. This mirrors a corrupt/truncated .mapgeo file and must produce a proper Err
    // rather than panicking on the slice index.
    geo.vertex_buffers[0].data.truncate(70);

    let err = decode_geometry(&geo).expect_err("truncated vertex buffer should error");
    match err {
        Error::VertexBufferTooShort {
            model,
            buffer_len,
            ..
        } => {
            assert_eq!(model, "MapGeo_Instance_0");
            assert_eq!(buffer_len, 70);
        }
        other => panic!("expected VertexBufferTooShort, got {other:?}"),
    }
}
