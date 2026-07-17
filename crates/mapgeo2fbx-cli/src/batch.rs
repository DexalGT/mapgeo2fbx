use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use mapgeo2fbx_core::decode::decode_geometry;
use mapgeo2fbx_core::fbx::write_fbx;
use mapgeo2fbx_core::info::{summarize, FileInfo};
use rayon::prelude::*;
use ritoshark::io::Parse;
use ritoshark::mapgeo::MapGeometry;
use walkdir::WalkDir;

use crate::ui::UiReporter;

pub struct BatchSummary {
    pub converted: usize,
    pub failed: Vec<(PathBuf, String)>,
}

fn parse_mapgeo(path: &Path) -> Result<MapGeometry> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    MapGeometry::from_bytes(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Parses a `.mapgeo` file and returns its info summary without writing any `.fbx`.
pub fn load_info(path: &Path) -> Result<FileInfo> {
    let geo = parse_mapgeo(path)?;
    let meshes = decode_geometry(&geo).with_context(|| format!("decoding {}", path.display()))?;
    let file_size = fs::metadata(path)
        .with_context(|| format!("reading metadata for {}", path.display()))?
        .len();
    Ok(summarize(&geo, &meshes, file_size))
}

/// Converts a single `.mapgeo` file to `.fbx`, writing next to the source unless
/// `output_override` is given. Returns the info summary of what was converted.
pub fn convert_one_file(input: &Path, output_override: Option<&Path>) -> Result<FileInfo> {
    let geo = parse_mapgeo(input)?;
    let meshes = decode_geometry(&geo).with_context(|| format!("decoding {}", input.display()))?;
    let file_size = fs::metadata(input)
        .with_context(|| format!("reading metadata for {}", input.display()))?
        .len();
    let info = summarize(&geo, &meshes, file_size);

    let output_path = output_override
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("fbx"));

    let mut file = fs::File::create(&output_path)
        .with_context(|| format!("creating {}", output_path.display()))?;
    write_fbx(&mut file, &meshes).with_context(|| format!("writing {}", output_path.display()))?;

    tracing::info!(input = %input.display(), output = %output_path.display(), "converted");
    Ok(info)
}

/// Recursively finds every `.mapgeo` under `root` and converts each next to its source,
/// in parallel. Errors on individual files are collected rather than aborting the whole batch.
pub fn convert_folder(root: &Path, ui: &UiReporter) -> BatchSummary {
    let files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("mapgeo"))
                .unwrap_or(false)
        })
        .collect();

    let failed: Mutex<Vec<(PathBuf, String)>> = Mutex::new(Vec::new());
    let converted_count = Mutex::new(0usize);

    files.par_iter().for_each(|path| {
        match convert_one_file(path, None) {
            Ok(_) => {
                *converted_count.lock().expect("lock poisoned") += 1;
                ui.converted(&path.display().to_string());
            }
            Err(e) => {
                failed
                    .lock()
                    .expect("lock poisoned")
                    .push((path.clone(), format!("{e:#}")));
                ui.failed(&path.display().to_string(), &format!("{e:#}"));
            }
        }
        ui.tick();
    });

    let converted = *converted_count.lock().expect("lock poisoned");
    BatchSummary {
        converted,
        failed: failed.into_inner().expect("lock poisoned"),
    }
}
