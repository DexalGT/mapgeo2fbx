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
        .ok_or_else(|| Error::MissingVertexDescription {
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

    // Index buffer stores absolute/global vertex indices; no per-submesh offset needed.
    let submeshes = model
        .submeshes
        .iter()
        .map(|sm| {
            let start = sm.index_start as usize;
            let count = sm.index_count as usize;
            let end = start
                .checked_add(count)
                .filter(|&end| end <= index_buffer.indices.len())
                .ok_or_else(|| Error::SubmeshIndexOutOfRange {
                    model: model.name.clone(),
                    start,
                    end: start.saturating_add(count),
                    buffer_len: index_buffer.indices.len(),
                })?;
            let indices = &index_buffer.indices[start..end];
            let triangle_indices = indices
                .chunks_exact(3)
                .map(|tri| [tri[0] as u32, tri[1] as u32, tri[2] as u32])
                .collect();
            Ok(DecodedSubmesh {
                name: sm.name.clone(),
                triangle_indices,
            })
        })
        .collect::<Result<Vec<_>>>()?;

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
            if offset + size > data.len() {
                return Err(Error::VertexBufferTooShort {
                    model: model.name.clone(),
                    offset,
                    needed: size,
                    buffer_len: data.len(),
                });
            }
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
