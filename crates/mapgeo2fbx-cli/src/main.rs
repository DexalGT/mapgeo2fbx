mod args;
mod banner;
mod batch;
mod interactive;
mod logging;
mod ui;

use std::path::{Path, PathBuf};

use anyhow::Result;
use args::{Cli, LogLevel};
use clap::Parser;
use colored::Colorize;
use mapgeo2fbx_core::info::FileInfo;

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let mode = detect_entry_mode(&raw);

    let result = match mode {
        EntryMode::Interactive => {
            logging::init(LogLevel::Normal, false);
            let r = interactive::run();
            if let Err(ref e) = r {
                eprintln!("{} {e:#}", "error:".bright_red().bold());
            }
            if r.is_err() {
                std::process::exit(1);
            }
            return;
        }
        EntryMode::DragDrop(path) => {
            logging::init(LogLevel::Normal, false);
            banner::print();
            let cli = Cli {
                input: path,
                output: None,
                info_only: false,
                verbose: false,
                json: false,
                log_level: LogLevel::Normal,
                no_pause: false,
            };
            run(&cli)
        }
        EntryMode::Flagged => {
            let cli = Cli::parse_from(&raw);
            logging::init(cli.log_level, cli.json);
            banner::print();
            let r = run(&cli);
            let no_pause = cli.no_pause || cli.json;
            if let Err(ref e) = r {
                eprintln!("{} {e:#}", "error:".bright_red().bold());
            }
            if !no_pause {
                pause();
            }
            if r.is_err() {
                std::process::exit(1);
            }
            return;
        }
    };

    if let Err(ref e) = result {
        eprintln!("{} {e:#}", "error:".bright_red().bold());
    }
    pause();
    if result.is_err() {
        std::process::exit(1);
    }
}

fn pause() {
    eprintln!();
    eprintln!("Press Enter to exit...");
    let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0u8]);
}

enum EntryMode {
    Interactive,
    DragDrop(PathBuf),
    Flagged,
}

/// Picks an entry mode from the raw argv, before clap runs — mirrors `hematite-cli`'s
/// `detect_entry_mode`. A single existing path with no flags covers both a dropped file and a
/// dropped folder; `run()` dispatches on `is_file()`/`is_dir()` from there.
fn detect_entry_mode(raw: &[String]) -> EntryMode {
    let user_args: Vec<&str> = raw.iter().skip(1).map(|s| s.as_str()).collect();

    if user_args.is_empty() {
        return EntryMode::Interactive;
    }

    if user_args.len() == 1 {
        let only = user_args[0];
        if !only.starts_with('-') && Path::new(only).exists() {
            return EntryMode::DragDrop(PathBuf::from(only));
        }
    }

    EntryMode::Flagged
}

pub fn run(cli: &Cli) -> Result<()> {
    if !cli.input.exists() {
        anyhow::bail!("input path does not exist: {}", cli.input.display());
    }

    if cli.input.is_file() {
        let info = if cli.info_only {
            batch::load_info(&cli.input)?
        } else {
            batch::convert_one_file(&cli.input, cli.output.as_deref())?
        };
        print_info(&info, cli.json, cli.verbose);
        Ok(())
    } else {
        if cli.output.is_some() {
            anyhow::bail!("--output is only valid when converting a single file, not a directory");
        }
        let mode = ui::Mode::from_json(cli.json);
        let file_count = walkdir::WalkDir::new(&cli.input)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("mapgeo"))
                    .unwrap_or(false)
            })
            .count() as u64;

        let reporter = ui::UiReporter::new(mode, file_count);
        let summary = batch::convert_folder(&cli.input, &reporter);
        reporter.finish();

        if cli.json {
            let payload = serde_json::json!({
                "converted": summary.converted,
                "failed": summary.failed.iter().map(|(p, e)| serde_json::json!({
                    "path": p.display().to_string(),
                    "error": e,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
        } else {
            println!(
                "\nconverted {} file(s), {} failed",
                summary.converted,
                summary.failed.len()
            );
        }

        if !summary.failed.is_empty() {
            anyhow::bail!("{} file(s) failed to convert", summary.failed.len());
        }
        Ok(())
    }
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
