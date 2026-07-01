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
