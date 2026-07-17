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
    geo.models
        .iter()
        .map(|model| decode_model(geo, model))
        .collect()
}

fn decode_model(geo: &MapGeometry, model: &MapModel) -> Result<DecodedMesh> {
    // A model's vertex attributes can be sharded across several vertex buffers (e.g. Position/UV in
    // one, Normal in another). Buffer `i` in `vertex_buffer_ids` is described by vertex description
    // `vertex_description_id + i` — the id stored on the model is the *base* declaration index, per
    // the OEGM format (see LeagueToolkit's `EnvironmentAssetMesh` reader).
    if model.vertex_buffer_ids.is_empty() {
        return Err(Error::MissingVertexBuffer {
            model: model.name.clone(),
            id: -1,
        });
    }

    let mut streams = Vec::with_capacity(model.vertex_buffer_ids.len());
    for (i, &vb_id) in model.vertex_buffer_ids.iter().enumerate() {
        let vertex_buffer =
            geo.vertex_buffers
                .get(vb_id as usize)
                .ok_or_else(|| Error::MissingVertexBuffer {
                    model: model.name.clone(),
                    id: vb_id,
                })?;
        let description_id = model.vertex_description_id as usize + i;
        let description = geo.vertex_descriptions.get(description_id).ok_or_else(|| {
            Error::MissingVertexDescription {
                model: model.name.clone(),
                id: description_id as i32,
            }
        })?;
        streams.push((description, vertex_buffer.data.as_slice()));
    }

    let index_buffer = geo
        .index_buffers
        .get(model.index_buffer_id as usize)
        .ok_or_else(|| Error::MissingIndexBuffer {
            model: model.name.clone(),
            id: model.index_buffer_id,
        })?;

    let vertices = decode_vertices(model, &streams)?;

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

/// Decodes every vertex by walking each (description, buffer) stream in parallel. For vertex `i`,
/// each stream is indexed at `i * stride` and its declared attributes are read from that stream's
/// buffer, so attributes sharded across buffers (Position in one, Normal in another) are all
/// gathered into a single [`DecodedVertex`].
fn decode_vertices(
    model: &MapModel,
    streams: &[(&VertexDescription, &[u8])],
) -> Result<Vec<DecodedVertex>> {
    let mut out = Vec::with_capacity(model.vertex_count as usize);

    for i in 0..model.vertex_count as usize {
        let mut position = Vec3::ZERO;
        let mut normal = Vec3::ZERO;
        let mut uv0 = Vec2::ZERO;

        for (description, data) in streams {
            let stride = description.vertex_size();
            let mut offset = i * stride;

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
        }

        let transformed_pos = model.transform.transform_point3(position);
        let transformed_normal = model
            .transform
            .transform_vector3(normal)
            .normalize_or_zero();

        out.push(DecodedVertex {
            position: transformed_pos,
            normal: transformed_normal,
            uv0,
        });
    }

    Ok(out)
}

fn read_f32(data: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

/// Reads a little-endian IEEE 754 half-precision (binary16) value and widens it to `f32`.
///
/// The OEGM "`Packed1616`"/"`Packed161616`" vertex formats store each component as a binary16
/// float, not a normalized integer — this matches the reference C# `VertexElementAccessor`, which
/// decodes them as `Half` tuples. Decoding is exact and needs no bounding box or scale/pivot.
fn read_f16(data: &[u8], offset: usize) -> f32 {
    let bits = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
    f16_bits_to_f32(bits)
}

fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = (bits >> 15) & 0x1;
    let exponent = (bits >> 10) & 0x1f;
    let mantissa = bits & 0x3ff;

    let sign_f = if sign == 1 { -1.0f32 } else { 1.0f32 };

    match exponent {
        0 => {
            // Subnormal (or zero): value = sign * 2^-14 * (mantissa / 1024).
            sign_f * 2.0f32.powi(-14) * (mantissa as f32 / 1024.0)
        }
        0x1f => {
            // Inf / NaN.
            if mantissa == 0 {
                sign_f * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => {
            // Normal: value = sign * 2^(exp-15) * (1 + mantissa/1024).
            sign_f * 2.0f32.powi(exponent as i32 - 15) * (1.0 + mantissa as f32 / 1024.0)
        }
    }
}

fn read_vec3(data: &[u8], offset: usize, format: ElementFormat) -> Result<Vec3> {
    match format {
        ElementFormat::XyzFloat32 => Ok(Vec3::new(
            read_f32(data, offset),
            read_f32(data, offset + 4),
            read_f32(data, offset + 8),
        )),
        // Three consecutive half-floats. The 8-byte slot's trailing 2 bytes are padding.
        ElementFormat::XyzPacked161616 => Ok(Vec3::new(
            read_f16(data, offset),
            read_f16(data, offset + 2),
            read_f16(data, offset + 4),
        )),
        other => Err(Error::UnsupportedVertexFormat {
            element: ElementName::Normal,
            format: other,
        }),
    }
}

fn read_vec2(data: &[u8], offset: usize, format: ElementFormat) -> Result<Vec2> {
    match format {
        ElementFormat::XyFloat32 => Ok(Vec2::new(
            read_f32(data, offset),
            read_f32(data, offset + 4),
        )),
        // Two consecutive half-floats.
        ElementFormat::XyPacked1616 => Ok(Vec2::new(
            read_f16(data, offset),
            read_f16(data, offset + 2),
        )),
        other => Err(Error::UnsupportedVertexFormat {
            element: ElementName::Texcoord0,
            format: other,
        }),
    }
}
