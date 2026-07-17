use std::collections::HashMap;
use std::io::Write;

use crate::decode::DecodedMesh;
use crate::error::{Error, Result};

/// Writes an ASCII FBX 7.4 scene containing one Model + Geometry pair per input mesh, with
/// per-submesh material assignment via a `ByPolygon`/`IndexToDirect` `LayerElementMaterial`.
/// Material nodes are deduplicated by name across the whole scene.
///
/// Targets Autodesk Maya. Two things are load-bearing and were verified against Maya 2023's own
/// importer (via `mayapy`):
///  1. Each `Model` MUST carry `P: "DefaultAttributeIndex", "int", "Integer", "",0`. Without it
///     Maya imports the transform but silently drops the connected mesh — the exact "empty folders
///     / nothing imports" symptom. This one property is the difference between 0 and 1 imported
///     meshes.
///  2. Geometry binds directly to its Model (Geometry->Model, Model->root, Material->Model). No
///     NodeAttribute — a decoded Maya export confirms mesh models don't use one, and adding a bogus
///     one does not help.
///
/// Large array payloads are wrapped across many short lines rather than one giant line, matching
/// the FBX SDK; Maya's ASCII tokenizer chokes on multi-megabyte single lines.
pub fn write_fbx<W: Write>(writer: &mut W, meshes: &[DecodedMesh]) -> Result<()> {
    let mut next_id: i64 = 1_000_000;
    let mut alloc_id = move || {
        let id = next_id;
        next_id += 1;
        id
    };

    // Model/Geometry ids per mesh first, then material ids, so ids stay stable and low.
    let mut model_geometry_ids = Vec::with_capacity(meshes.len());
    for _mesh in meshes {
        let model_id = alloc_id();
        let geometry_id = alloc_id();
        model_geometry_ids.push((model_id, geometry_id));
    }

    // A stable object id per unique material name across all meshes.
    let mut material_ids: HashMap<String, i64> = HashMap::new();
    for mesh in meshes {
        for submesh in &mesh.submeshes {
            material_ids
                .entry(submesh.name.clone())
                .or_insert_with(&mut alloc_id);
        }
    }

    write_header(writer)?;
    write_global_settings(writer)?;
    write_documents(writer)?;
    writeln!(writer, "References:  {{\n}}\n")?;
    write_definitions(writer, meshes.len(), material_ids.len())?;

    writeln!(writer, "Objects:  {{")?;
    for (mesh, (model_id, geometry_id)) in meshes.iter().zip(&model_geometry_ids) {
        write_model(writer, *model_id, &mesh.name)?;
        write_geometry(writer, *geometry_id, mesh)?;
    }
    let mut sorted_materials: Vec<(&String, &i64)> = material_ids.iter().collect();
    sorted_materials.sort_by_key(|(_, id)| **id);
    for (name, id) in sorted_materials {
        write_material(writer, *id, name)?;
    }
    writeln!(writer, "}}\n")?;

    writeln!(writer, "Connections:  {{")?;
    for (mesh, (model_id, geometry_id)) in meshes.iter().zip(&model_geometry_ids) {
        writeln!(writer, "\tC: \"OO\",{geometry_id},{model_id}")?;
        writeln!(writer, "\tC: \"OO\",{model_id},0")?;
        let mut seen = std::collections::HashSet::new();
        for submesh in &mesh.submeshes {
            if seen.insert(&submesh.name) {
                let material_id = material_ids[&submesh.name];
                writeln!(writer, "\tC: \"OO\",{material_id},{model_id}")?;
            }
        }
    }
    writeln!(writer, "}}\n")?;

    Ok(())
}

fn write_header<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "; FBX 7.4.0 project file")?;
    writeln!(writer, "FBXHeaderExtension:  {{")?;
    writeln!(writer, "\tFBXHeaderVersion: 1003")?;
    writeln!(writer, "\tFBXVersion: 7400")?;
    writeln!(writer, "\tCreationTimeStamp:  {{")?;
    writeln!(writer, "\t\tVersion: 1000")?;
    writeln!(writer, "\t\tYear: 1970")?;
    writeln!(writer, "\t\tMonth: 1")?;
    writeln!(writer, "\t\tDay: 1")?;
    writeln!(writer, "\t\tHour: 0")?;
    writeln!(writer, "\t\tMinute: 0")?;
    writeln!(writer, "\t\tSecond: 0")?;
    writeln!(writer, "\t\tMillisecond: 0")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "\tCreator: \"mapgeo2fbx\"")?;
    writeln!(writer, "}}\n")?;
    writeln!(writer, "CreationTime: \"1970-01-01 00:00:00:000\"")?;
    writeln!(writer, "Creator: \"mapgeo2fbx\"\n")?;
    Ok(())
}

fn write_global_settings<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "GlobalSettings:  {{")?;
    writeln!(writer, "\tVersion: 1000")?;
    writeln!(writer, "\tProperties70:  {{")?;
    writeln!(writer, "\t\tP: \"UpAxis\", \"int\", \"Integer\", \"\",1")?;
    writeln!(writer, "\t\tP: \"UpAxisSign\", \"int\", \"Integer\", \"\",1")?;
    writeln!(writer, "\t\tP: \"FrontAxis\", \"int\", \"Integer\", \"\",2")?;
    writeln!(writer, "\t\tP: \"FrontAxisSign\", \"int\", \"Integer\", \"\",1")?;
    writeln!(writer, "\t\tP: \"CoordAxis\", \"int\", \"Integer\", \"\",0")?;
    writeln!(writer, "\t\tP: \"CoordAxisSign\", \"int\", \"Integer\", \"\",1")?;
    writeln!(
        writer,
        "\t\tP: \"UnitScaleFactor\", \"double\", \"Number\", \"\",1"
    )?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "}}\n")?;
    Ok(())
}

fn write_documents<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "Documents:  {{")?;
    writeln!(writer, "\tCount: 1")?;
    writeln!(writer, "\tDocument: 1000000000, \"\", \"Scene\" {{")?;
    writeln!(writer, "\t\tProperties70:  {{")?;
    writeln!(writer, "\t\t\tP: \"SourceObject\", \"object\", \"\", \"\"")?;
    writeln!(
        writer,
        "\t\t\tP: \"ActiveAnimStackName\", \"KString\", \"\", \"\", \"\""
    )?;
    writeln!(writer, "\t\t}}")?;
    writeln!(writer, "\t\tRootNode: 0")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "}}\n")?;
    Ok(())
}

fn write_definitions<W: Write>(
    writer: &mut W,
    model_count: usize,
    material_count: usize,
) -> Result<()> {
    // One GlobalSettings + one Model and Geometry per mesh + one Material per unique name.
    let total = 1 + model_count * 2 + material_count;
    writeln!(writer, "Definitions:  {{")?;
    writeln!(writer, "\tVersion: 100")?;
    writeln!(writer, "\tCount: {total}")?;
    writeln!(writer, "\tObjectType: \"GlobalSettings\" {{")?;
    writeln!(writer, "\t\tCount: 1")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "\tObjectType: \"Model\" {{")?;
    writeln!(writer, "\t\tCount: {model_count}")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "\tObjectType: \"Geometry\" {{")?;
    writeln!(writer, "\t\tCount: {model_count}")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "\tObjectType: \"Material\" {{")?;
    writeln!(writer, "\t\tCount: {material_count}")?;
    writeln!(writer, "\t}}")?;
    writeln!(writer, "}}\n")?;
    Ok(())
}

fn write_model<W: Write>(writer: &mut W, model_id: i64, name: &str) -> Result<()> {
    // ASCII names use the "Class::name" form (class word first, literal "::").
    writeln!(writer, "\tModel: {model_id}, \"Model::{name}\", \"Mesh\" {{")?;
    writeln!(writer, "\t\tVersion: 232")?;
    writeln!(writer, "\t\tProperties70:  {{")?;
    // REQUIRED by Maya: binds the connected Geometry to this transform. Omitting it makes Maya
    // drop the mesh silently. Verified against Maya 2023's importer.
    writeln!(
        writer,
        "\t\t\tP: \"DefaultAttributeIndex\", \"int\", \"Integer\", \"\",0"
    )?;
    writeln!(
        writer,
        "\t\t\tP: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",0,0,0"
    )?;
    writeln!(
        writer,
        "\t\t\tP: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",0,0,0"
    )?;
    writeln!(
        writer,
        "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1"
    )?;
    writeln!(writer, "\t\t}}")?;
    writeln!(writer, "\t\tShading: T")?;
    writeln!(writer, "\t\tCulling: \"CullingOff\"")?;
    writeln!(writer, "\t}}\n")?;
    Ok(())
}

fn write_geometry<W: Write>(writer: &mut W, geometry_id: i64, mesh: &DecodedMesh) -> Result<()> {
    // Vertex world transform is already baked into position/normal by decode::decode_geometry,
    // so the Model's Lcl Translation/Rotation/Scaling stay identity.
    let vertex_count = mesh.vertices.len();

    let mut ordered_material_names: Vec<&String> = Vec::new();
    let mut name_to_local_index: HashMap<&str, u32> = HashMap::new();
    for submesh in &mesh.submeshes {
        if !name_to_local_index.contains_key(submesh.name.as_str()) {
            name_to_local_index.insert(submesh.name.as_str(), ordered_material_names.len() as u32);
            ordered_material_names.push(&submesh.name);
        }
    }

    let mut polygon_vertex_index: Vec<i64> = Vec::new();
    let mut material_per_polygon: Vec<u32> = Vec::new();
    for submesh in &mesh.submeshes {
        let local_material_index = name_to_local_index[submesh.name.as_str()];
        for tri in &submesh.triangle_indices {
            polygon_vertex_index.push(tri[0] as i64);
            polygon_vertex_index.push(tri[1] as i64);
            polygon_vertex_index.push(-(tri[2] as i64) - 1);
            material_per_polygon.push(local_material_index);
        }
    }
    let polygon_count = material_per_polygon.len();

    writeln!(
        writer,
        "\tGeometry: {geometry_id}, \"Geometry::{}\", \"Mesh\" {{",
        mesh.name
    )?;

    let vertex_floats: Vec<String> = mesh
        .vertices
        .iter()
        .flat_map(|v| [v.position.x, v.position.y, v.position.z])
        .map(format_f32)
        .collect();
    writeln!(writer, "\t\tVertices: *{} {{", vertex_count * 3)?;
    write_array(writer, "\t\t\t", &vertex_floats)?;
    writeln!(writer, "\t\t}}")?;

    let index_strs: Vec<String> = polygon_vertex_index.iter().map(|i| i.to_string()).collect();
    writeln!(
        writer,
        "\t\tPolygonVertexIndex: *{} {{",
        polygon_vertex_index.len()
    )?;
    write_array(writer, "\t\t\t", &index_strs)?;
    writeln!(writer, "\t\t}}")?;
    writeln!(writer, "\t\tGeometryVersion: 124")?;

    // Normals: ByPolygonVertex/Direct, one triplet per (polygon, vertex-in-polygon).
    let mut normal_floats: Vec<String> = Vec::new();
    for submesh in &mesh.submeshes {
        for tri in &submesh.triangle_indices {
            for &vi in tri {
                let v =
                    mesh.vertices
                        .get(vi as usize)
                        .ok_or_else(|| Error::VertexIndexOutOfRange {
                            mesh: mesh.name.clone(),
                            index: vi,
                            vertex_count: mesh.vertices.len(),
                        })?;
                normal_floats.push(format_f32(v.normal.x));
                normal_floats.push(format_f32(v.normal.y));
                normal_floats.push(format_f32(v.normal.z));
            }
        }
    }
    writeln!(writer, "\t\tLayerElementNormal: 0 {{")?;
    writeln!(writer, "\t\t\tVersion: 101")?;
    writeln!(writer, "\t\t\tName: \"\"")?;
    writeln!(writer, "\t\t\tMappingInformationType: \"ByPolygonVertex\"")?;
    writeln!(writer, "\t\t\tReferenceInformationType: \"Direct\"")?;
    writeln!(writer, "\t\t\tNormals: *{} {{", normal_floats.len())?;
    write_array(writer, "\t\t\t\t", &normal_floats)?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t}}")?;

    // UVs: Direct per-vertex array + IndexToDirect index list matching PolygonVertexIndex order.
    let uv_floats: Vec<String> = mesh
        .vertices
        .iter()
        .flat_map(|v| [v.uv0.x, v.uv0.y])
        .map(format_f32)
        .collect();
    let mut uv_index: Vec<u32> = Vec::new();
    for submesh in &mesh.submeshes {
        for tri in &submesh.triangle_indices {
            uv_index.extend_from_slice(tri);
        }
    }
    let uv_index_strs: Vec<String> = uv_index.iter().map(|i| i.to_string()).collect();
    writeln!(writer, "\t\tLayerElementUV: 0 {{")?;
    writeln!(writer, "\t\t\tVersion: 101")?;
    writeln!(writer, "\t\t\tName: \"\"")?;
    writeln!(writer, "\t\t\tMappingInformationType: \"ByPolygonVertex\"")?;
    writeln!(writer, "\t\t\tReferenceInformationType: \"IndexToDirect\"")?;
    writeln!(writer, "\t\t\tUV: *{} {{", uv_floats.len())?;
    write_array(writer, "\t\t\t\t", &uv_floats)?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t\tUVIndex: *{} {{", uv_index_strs.len())?;
    write_array(writer, "\t\t\t\t", &uv_index_strs)?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t}}")?;

    let material_index_strs: Vec<String> =
        material_per_polygon.iter().map(|i| i.to_string()).collect();
    writeln!(writer, "\t\tLayerElementMaterial: 0 {{")?;
    writeln!(writer, "\t\t\tVersion: 101")?;
    writeln!(writer, "\t\t\tName: \"\"")?;
    writeln!(writer, "\t\t\tMappingInformationType: \"ByPolygon\"")?;
    writeln!(writer, "\t\t\tReferenceInformationType: \"IndexToDirect\"")?;
    writeln!(writer, "\t\t\tMaterials: *{polygon_count} {{")?;
    write_array(writer, "\t\t\t\t", &material_index_strs)?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t}}")?;

    writeln!(writer, "\t\tLayer: 0 {{")?;
    writeln!(writer, "\t\t\tVersion: 100")?;
    writeln!(writer, "\t\t\tLayerElement:  {{")?;
    writeln!(writer, "\t\t\t\tType: \"LayerElementNormal\"")?;
    writeln!(writer, "\t\t\t\tTypedIndex: 0")?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t\tLayerElement:  {{")?;
    writeln!(writer, "\t\t\t\tType: \"LayerElementMaterial\"")?;
    writeln!(writer, "\t\t\t\tTypedIndex: 0")?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t\tLayerElement:  {{")?;
    writeln!(writer, "\t\t\t\tType: \"LayerElementUV\"")?;
    writeln!(writer, "\t\t\t\tTypedIndex: 0")?;
    writeln!(writer, "\t\t\t}}")?;
    writeln!(writer, "\t\t}}")?;

    writeln!(writer, "\t}}\n")?;
    Ok(())
}

fn write_material<W: Write>(writer: &mut W, material_id: i64, name: &str) -> Result<()> {
    writeln!(
        writer,
        "\tMaterial: {material_id}, \"Material::{name}\", \"\" {{"
    )?;
    writeln!(writer, "\t\tVersion: 102")?;
    writeln!(writer, "\t\tShadingModel: \"Lambert\"")?;
    writeln!(writer, "\t\tMultiLayer: 0")?;
    writeln!(writer, "\t\tProperties70:  {{")?;
    writeln!(
        writer,
        "\t\t\tP: \"DiffuseColor\", \"Color\", \"\", \"A\",0.8,0.8,0.8"
    )?;
    writeln!(writer, "\t\t}}")?;
    writeln!(writer, "\t}}\n")?;
    Ok(())
}

/// Writes an FBX `a:` array value, wrapping across many lines so no single line grows unbounded.
/// Maya's ASCII importer mishandles multi-megabyte single lines; the FBX SDK wraps similarly.
fn write_array<W: Write>(writer: &mut W, indent: &str, values: &[String]) -> Result<()> {
    const PER_LINE: usize = 64;
    if values.is_empty() {
        writeln!(writer, "{indent}a: ")?;
        return Ok(());
    }
    let total = values.len();
    for (chunk_index, chunk) in values.chunks(PER_LINE).enumerate() {
        let joined = chunk.join(",");
        let is_last_chunk = (chunk_index + 1) * PER_LINE >= total;
        let sep = if is_last_chunk { "" } else { "," };
        if chunk_index == 0 {
            writeln!(writer, "{indent}a: {joined}{sep}")?;
        } else {
            writeln!(writer, "{indent}{joined}{sep}")?;
        }
    }
    Ok(())
}

fn format_f32(v: f32) -> String {
    // FBX ASCII tolerates plain decimal formatting; avoid scientific notation which some
    // importers mis-parse.
    format!("{v}")
}
