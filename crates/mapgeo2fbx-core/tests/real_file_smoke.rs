use std::path::Path;

use mapgeo2fbx_core::decode::decode_geometry;
use mapgeo2fbx_core::fbx::write_fbx;
use mapgeo2fbx_core::info::summarize;
use ritoshark::io::Parse;
use ritoshark::mapgeo::MapGeometry;

/// Exercises the full decode -> summarize -> write_fbx pipeline against a real map file.
/// Skips cleanly (rather than failing) if the sample isn't present, matching the convention
/// `RitoShark-Crates` itself uses for real-file tests that depend on non-committed game data.
#[test]
fn converts_real_sample_file() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sample-Files/bloom.mapgeo");
    if !path.exists() {
        eprintln!("skipping: sample file not present at {}", path.display());
        return;
    }

    let bytes = std::fs::read(&path).expect("read sample file");
    let geo = MapGeometry::from_bytes(&bytes).expect("parse real mapgeo file");
    let meshes = decode_geometry(&geo).expect("decode real mapgeo file");

    assert!(!meshes.is_empty(), "expected at least one decoded mesh");
    for mesh in &meshes {
        assert!(
            !mesh.vertices.is_empty(),
            "mesh {} has no vertices",
            mesh.name
        );
    }

    let info = summarize(&geo, &meshes, bytes.len() as u64);
    assert_eq!(info.model_count, meshes.len());
    assert!(info.total_vertices > 0);

    let mut buf = Vec::new();
    write_fbx(&mut buf, &meshes).expect("write real mapgeo file to fbx");

    // Output is binary FBX; assert the magic and that the key node names appear as byte substrings.
    assert!(buf.starts_with(b"Kaydara FBX Binary  "));
    let contains = |needle: &[u8]| buf.windows(needle.len()).any(|w| w == needle);
    assert!(contains(b"FBXHeaderExtension"));
    assert!(contains(b"Objects"));
    assert!(contains(b"Connections"));
}
