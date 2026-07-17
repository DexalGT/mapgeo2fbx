use mapgeo2fbx_core::decode::decode_geometry;
use mapgeo2fbx_core::Error;
use ritoshark::mapgeo::{
    AssetChannel, ElementFormat, ElementName, IndexBuffer, MapGeometry, MapModel, Submesh,
    VertexBuffer, VertexDescription, VertexElement, VertexUsage,
};
use ritoshark::math::{Aabb, Mat4, Vec2, Vec3};

/// IEEE 754 half-precision (binary16) bit patterns for the values used in the packed-format tests.
const H_0_0: u16 = 0x0000; //  0.0
const H_0_5: u16 = 0x3800; //  0.5
const H_1_0: u16 = 0x3C00; //  1.0
const H_2_0: u16 = 0x4000; //  2.0
const H_NEG_1_0: u16 = 0xBC00; // -1.0

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
            model, buffer_len, ..
        } => {
            assert_eq!(model, "MapGeo_Instance_0");
            assert_eq!(buffer_len, 70);
        }
        other => panic!("expected VertexBufferTooShort, got {other:?}"),
    }
}

/// A single vertex whose Normal is `XyzPacked161616` (three IEEE-754 half-floats) and whose
/// Texcoord0 is `XyPacked1616` (two half-floats). Position stays `XyzFloat32`. This mirrors the
/// real `carousel_set17.mapgeo` layout, where only Normal/UV are half-packed and Position is a
/// plain float. The reference C# `VertexElementAccessor` decodes these formats as `Half` tuples,
/// so the packed bytes are literal binary16 values — no bounding-box scale/pivot involved.
fn packed_half_geometry() -> MapGeometry {
    let elements = vec![
        VertexElement {
            name: ElementName::Position,
            format: ElementFormat::XyzFloat32,
        },
        VertexElement {
            name: ElementName::Normal,
            format: ElementFormat::XyzPacked161616,
        },
        VertexElement {
            name: ElementName::Texcoord0,
            format: ElementFormat::XyPacked1616,
        },
    ];

    // Stride = 12 (pos) + 8 (normal, only 6 bytes used + 2 padding) + 4 (uv) = 24 bytes.
    let mut data = Vec::new();
    // Position (plain floats)
    for v in [3.0f32, 4.0, 5.0] {
        data.extend_from_slice(&v.to_le_bytes());
    }
    // Normal (0.0, 1.0, -1.0) as three half-floats + one padding half to fill the 8-byte slot.
    for h in [H_0_0, H_1_0, H_NEG_1_0, H_0_0] {
        data.extend_from_slice(&h.to_le_bytes());
    }
    // Texcoord0 (0.5, 2.0) as two half-floats.
    for h in [H_0_5, H_2_0] {
        data.extend_from_slice(&h.to_le_bytes());
    }
    assert_eq!(data.len(), 24);

    let model = MapModel {
        name: "MapGeo_Instance_0".to_string(),
        vertex_count: 1,
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
            max_vertex: 0,
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
            elements,
        }],
        vertex_buffers: vec![VertexBuffer { layer: 0, data }],
        index_buffers: vec![IndexBuffer {
            layer: 0,
            indices: vec![0, 0, 0],
        }],
        models: vec![model],
        scene_graphs: vec![],
        planar_reflectors: vec![],
    }
}

#[test]
fn decodes_packed_half_normal_and_uv() {
    let geo = packed_half_geometry();
    let meshes = decode_geometry(&geo).expect("decode should succeed");

    let v = meshes[0].vertices[0];
    assert_eq!(v.position, Vec3::new(3.0, 4.0, 5.0));
    // Normal is normalized on decode; (0, 1, -1) normalizes to (0, 1/√2, -1/√2).
    let inv_sqrt2 = 1.0f32 / 2.0f32.sqrt();
    assert!((v.normal - Vec3::new(0.0, inv_sqrt2, -inv_sqrt2)).length() < 1e-4);
    assert_eq!(v.uv0, Vec2::new(0.5, 2.0));
}

/// A model whose attributes are sharded across two vertex buffers, exactly like real map
/// geometry: buffer 0 (description `base_desc`) holds Position + Texcoord0, buffer 1
/// (description `base_desc + 1`) holds the Normal. The decoder must locate each attribute in
/// whichever buffer's description declares it, not apply a single description to the first buffer.
fn two_buffer_geometry() -> MapGeometry {
    // Description 0: Position (XyzFloat32) + Texcoord0 (XyPacked1616). Stride 16.
    let desc0 = VertexDescription {
        usage: VertexUsage::Static,
        elements: vec![
            VertexElement {
                name: ElementName::Position,
                format: ElementFormat::XyzFloat32,
            },
            VertexElement {
                name: ElementName::Texcoord0,
                format: ElementFormat::XyPacked1616,
            },
        ],
    };
    // Description 1: Normal (XyzPacked161616). Stride 8.
    let desc1 = VertexDescription {
        usage: VertexUsage::Static,
        elements: vec![VertexElement {
            name: ElementName::Normal,
            format: ElementFormat::XyzPacked161616,
        }],
    };

    // Buffer 0: one vertex — position (1,2,3) + uv (1.0, 0.5).
    let mut buf0 = Vec::new();
    for v in [1.0f32, 2.0, 3.0] {
        buf0.extend_from_slice(&v.to_le_bytes());
    }
    for h in [H_1_0, H_0_5] {
        buf0.extend_from_slice(&h.to_le_bytes());
    }
    assert_eq!(buf0.len(), 16);

    // Buffer 1: one vertex — normal (1,0,0) + padding half.
    let mut buf1 = Vec::new();
    for h in [H_1_0, H_0_0, H_0_0, H_0_0] {
        buf1.extend_from_slice(&h.to_le_bytes());
    }
    assert_eq!(buf1.len(), 8);

    let model = MapModel {
        name: "MapGeo_Instance_0".to_string(),
        vertex_count: 1,
        // base description id; buffer i uses description base + i.
        vertex_description_id: 0,
        vertex_buffer_ids: vec![0, 1],
        index_count: 3,
        index_buffer_id: 0,
        layer: 0,
        unknown_v18: 0,
        bucket_grid_hash: 0,
        submeshes: vec![Submesh {
            hash: 0,
            name: "Materials/Rock".to_string(),
            index_start: 0,
            index_count: 3,
            min_vertex: 0,
            max_vertex: 0,
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
        vertex_descriptions: vec![desc0, desc1],
        vertex_buffers: vec![
            VertexBuffer {
                layer: 0,
                data: buf0,
            },
            VertexBuffer {
                layer: 0,
                data: buf1,
            },
        ],
        index_buffers: vec![IndexBuffer {
            layer: 0,
            indices: vec![0, 0, 0],
        }],
        models: vec![model],
        scene_graphs: vec![],
        planar_reflectors: vec![],
    }
}

#[test]
fn decodes_attributes_split_across_two_vertex_buffers() {
    let geo = two_buffer_geometry();
    let meshes = decode_geometry(&geo).expect("decode should succeed");

    let v = meshes[0].vertices[0];
    assert_eq!(v.position, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(v.uv0, Vec2::new(1.0, 0.5));
    // Normal came from the second buffer; (1,0,0) normalizes to itself.
    assert!((v.normal - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4);
}
