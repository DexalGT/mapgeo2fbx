mod args;
mod banner;
mod logging;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use args::Cli;
use clap::Parser;
use colored::Colorize;
use mapgeo2fbx_core::decode::decode_geometry;
use mapgeo2fbx_core::fbx::write_fbx;
use mapgeo2fbx_core::info::{summarize, FileInfo};
use ritoshark::io::Parse;
use ritoshark::mapgeo::MapGeometry;

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let cli = Cli::parse_from(raw);

    logging::init(cli.log_level, cli.json);
    banner::print();

    let result = run(&cli);
    if let Err(ref e) = result {
        eprintln!("{} {e:#}", "error:".bright_red().bold());
    }

    if !cli.no_pause && !cli.json {
        eprintln!();
        eprintln!("Press Enter to exit...");
        let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0u8]);
    }

    if result.is_err() {
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    if !cli.input.exists() {
        anyhow::bail!("input path does not exist: {}", cli.input.display());
    }

    if cli.input.is_file() {
        let info = if cli.info_only {
            load_info(&cli.input)?
        } else {
            convert_one_file(&cli.input, cli.output.as_deref())?
        };
        print_info(&info, cli.json, cli.verbose);
        Ok(())
    } else {
        anyhow::bail!(
            "'{}' is a directory — batch conversion is handled by the CLI's batch mode, not implemented in this task yet",
            cli.input.display()
        );
    }
}

/// Parses a `.mapgeo` file and returns its info summary without writing any `.fbx`.
fn load_info(path: &Path) -> Result<FileInfo> {
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

fn parse_mapgeo(path: &Path) -> Result<MapGeometry> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    MapGeometry::from_bytes(&bytes).with_context(|| format!("parsing {}", path.display()))
}

fn print_info(info: &FileInfo, json: bool, verbose: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(info).expect("FileInfo serializes"));
        return;
    }
    if verbose {
        println!("{info}");
    } else {
        println!(
            "mapgeo version: {} | models: {} | vertices: {} | triangles: {} | materials: {} | size: {} bytes",
            info.version,
            info.model_count,
            info.total_vertices,
            info.total_triangles,
            info.unique_material_count,
            info.file_size_bytes
        );
    }
}
