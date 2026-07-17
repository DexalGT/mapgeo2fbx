use std::fs;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

/// Builds the smallest valid OEGM v17 file: one Position-only vertex declaration, one
/// vertex/index buffer, one model, no submeshes. Mirrors `rs_mapgeo`'s own `tests/smoke.rs`
/// fixture so the byte layout is known-good against the real parser.
fn minimal_v17_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"OEGM");
    b.extend_from_slice(&17u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes()); // texture overrides

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 vertex decl
    b.extend_from_slice(&0u32.to_le_bytes()); // usage = Static
    b.extend_from_slice(&1u32.to_le_bytes()); // element count
    b.extend_from_slice(&0u32.to_le_bytes()); // name = Position
    b.extend_from_slice(&2u32.to_le_bytes()); // format = XYZ_Float32
    for _ in 0..14 {
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&3u32.to_le_bytes());
    }

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 vertex buffer
    b.push(0u8);
    b.extend_from_slice(&12u32.to_le_bytes());
    b.extend_from_slice(&1.0f32.to_le_bytes());
    b.extend_from_slice(&2.0f32.to_le_bytes());
    b.extend_from_slice(&3.0f32.to_le_bytes());

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 index buffer
    b.push(0u8);
    b.extend_from_slice(&6u32.to_le_bytes());
    for i in 0u16..3 {
        b.extend_from_slice(&i.to_le_bytes());
    }

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 model
    b.extend_from_slice(&1u32.to_le_bytes()); // vertex count
    b.extend_from_slice(&1u32.to_le_bytes()); // vertex buffer count
    b.extend_from_slice(&0u32.to_le_bytes()); // vertex desc id
    b.extend_from_slice(&0i32.to_le_bytes()); // vertex buffer id
    b.extend_from_slice(&3u32.to_le_bytes()); // index count
    b.extend_from_slice(&0i32.to_le_bytes()); // index buffer id
    b.push(0u8); // layer
    b.extend_from_slice(&0u32.to_le_bytes()); // bucket grid hash
    b.extend_from_slice(&0u32.to_le_bytes()); // submesh count
    b.push(0u8); // disable backface culling
    for v in [0.0f32, 0.0, 0.0, 1.0, 1.0, 1.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    let identity: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    for v in identity {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.push(31u8); // quality
    b.push(0u8); // layer transition behavior
    b.extend_from_slice(&0u16.to_le_bytes()); // render flags
    b.extend_from_slice(&0u32.to_le_bytes()); // baked light path
    for v in [1.0f32, 1.0, 0.0, 0.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0u32.to_le_bytes()); // stationary light path
    for v in [0.0f32, 0.0, 0.0, 0.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0u32.to_le_bytes()); // model texture overrides
    for v in [0.0f32, 0.0, 0.0, 0.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }

    b.extend_from_slice(&1u32.to_le_bytes()); // 1 disabled scene graph
    b.extend_from_slice(&0u32.to_le_bytes());
    for v in [0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0u16.to_le_bytes());
    b.push(1u8); // is_disabled
    b.push(0u8);
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());

    b.extend_from_slice(&0u32.to_le_bytes()); // 0 planar reflectors

    b
}

#[test]
fn converts_minimal_mapgeo_to_fbx() {
    let dir = tempdir().expect("tempdir");
    let input_path = dir.path().join("test.mapgeo");
    fs::write(&input_path, minimal_v17_bytes()).expect("write fixture");

    Command::cargo_bin("mapgeo2fbx")
        .expect("binary exists")
        .arg(&input_path)
        .arg("--no-pause")
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"model_count\": 1"));

    let output_path = dir.path().join("test.fbx");
    assert!(
        output_path.exists(),
        "expected test.fbx to be written next to test.mapgeo"
    );
    let fbx_text = fs::read_to_string(&output_path).expect("read fbx output");
    assert!(fbx_text.contains("FBXHeaderExtension"));
    assert!(fbx_text.contains(r#"P: "DefaultAttributeIndex", "int", "Integer", "",0"#));
}

#[test]
fn info_only_does_not_write_fbx() {
    let dir = tempdir().expect("tempdir");
    let input_path = dir.path().join("test.mapgeo");
    fs::write(&input_path, minimal_v17_bytes()).expect("write fixture");

    Command::cargo_bin("mapgeo2fbx")
        .expect("binary exists")
        .arg(&input_path)
        .arg("--info-only")
        .arg("--no-pause")
        .arg("--json")
        .assert()
        .success();

    let output_path = dir.path().join("test.fbx");
    assert!(
        !output_path.exists(),
        "--info-only must not write an fbx file"
    );
}

#[test]
fn converts_folder_recursively() {
    let dir = tempdir().expect("tempdir");
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).expect("mkdir nested");

    fs::write(dir.path().join("a.mapgeo"), minimal_v17_bytes()).expect("write a");
    fs::write(nested.join("b.mapgeo"), minimal_v17_bytes()).expect("write b");

    Command::cargo_bin("mapgeo2fbx")
        .expect("binary exists")
        .arg(dir.path())
        .arg("--no-pause")
        .arg("--json")
        .assert()
        .success()
        .stdout(contains("\"converted\": 2"));

    assert!(dir.path().join("a.fbx").exists());
    assert!(nested.join("b.fbx").exists());
}

#[test]
fn flagged_mode_with_output_override_works() {
    let dir = tempdir().expect("tempdir");
    let input_path = dir.path().join("test.mapgeo");
    fs::write(&input_path, minimal_v17_bytes()).expect("write fixture");
    let output_path = dir.path().join("custom_name.fbx");

    Command::cargo_bin("mapgeo2fbx")
        .expect("binary exists")
        .arg(&input_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--no-pause")
        .arg("--json")
        .assert()
        .success();

    assert!(output_path.exists());
    assert!(!dir.path().join("test.fbx").exists());
}
