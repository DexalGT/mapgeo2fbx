use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "mapgeo2fbx")]
#[command(about = "Convert League of Legends .mapgeo map geometry to ASCII .fbx")]
#[command(version)]
pub struct Cli {
    /// Input .mapgeo file, or a directory to recursively convert every .mapgeo inside.
    pub input: PathBuf,

    /// Output .fbx path override. Only valid when `input` is a single file.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Print the info summary and exit without writing any .fbx.
    #[arg(long)]
    pub info_only: bool,

    /// Include a per-model breakdown in the info summary.
    #[arg(short, long)]
    pub verbose: bool,

    /// Machine-readable JSON output for the info summary and conversion result.
    #[arg(long)]
    pub json: bool,

    /// Logging verbosity, independent of --verbose (which controls info detail).
    #[arg(long, value_enum, default_value = "normal")]
    pub log_level: LogLevel,

    /// Skip the "Press Enter to exit" pause at the end.
    #[arg(long)]
    pub no_pause: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LogLevel {
    Quiet,
    Normal,
    Verbose,
    Trace,
}
