use std::collections::HashSet;
use std::fmt;

use ritoshark::mapgeo::MapGeometry;

use crate::decode::DecodedMesh;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub materials: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileInfo {
    pub version: u32,
    pub model_count: usize,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub unique_material_count: usize,
    pub file_size_bytes: u64,
    pub models: Vec<ModelInfo>,
}

/// Builds a summary from the raw parsed geometry (for the version/file-size header) plus the
/// already-decoded meshes (for vertex/triangle/material counts, so the numbers match exactly
/// what the FBX writer will emit).
pub fn summarize(geo: &MapGeometry, meshes: &[DecodedMesh], file_size_bytes: u64) -> FileInfo {
    let mut all_materials: HashSet<&str> = HashSet::new();
    let mut total_vertices = 0usize;
    let mut total_triangles = 0usize;

    let models: Vec<ModelInfo> = meshes
        .iter()
        .map(|mesh| {
            let mut model_materials: Vec<String> = Vec::new();
            let mut seen = HashSet::new();
            let mut triangle_count = 0usize;
            for submesh in &mesh.submeshes {
                triangle_count += submesh.triangle_indices.len();
                if seen.insert(submesh.name.as_str()) {
                    model_materials.push(submesh.name.clone());
                }
                all_materials.insert(submesh.name.as_str());
            }
            total_vertices += mesh.vertices.len();
            total_triangles += triangle_count;

            ModelInfo {
                name: mesh.name.clone(),
                vertex_count: mesh.vertices.len(),
                triangle_count,
                materials: model_materials,
            }
        })
        .collect();

    FileInfo {
        version: geo.version,
        model_count: models.len(),
        total_vertices,
        total_triangles,
        unique_material_count: all_materials.len(),
        file_size_bytes,
        models,
    }
}

impl fmt::Display for FileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "mapgeo version: {}", self.version)?;
        writeln!(f, "models: {}", self.model_count)?;
        writeln!(f, "total vertices: {}", self.total_vertices)?;
        writeln!(f, "total triangles: {}", self.total_triangles)?;
        writeln!(f, "unique materials: {}", self.unique_material_count)?;
        write!(f, "file size: {} bytes", self.file_size_bytes)?;
        for model in &self.models {
            writeln!(f)?;
            write!(
                f,
                "  - {}: {} verts, {} tris, materials: [{}]",
                model.name,
                model.vertex_count,
                model.triangle_count,
                model.materials.join(", ")
            )?;
        }
        Ok(())
    }
}
