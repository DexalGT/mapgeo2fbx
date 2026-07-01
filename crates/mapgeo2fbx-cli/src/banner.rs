use colored::Colorize;

const TAGLINE: &str = "MapGeo -> FBX converter";

/// Prints the splash to stderr (stdout stays clean for --json / piped output).
pub fn print() {
    eprintln!();
    eprintln!("{}", "  mapgeo2fbx".bright_cyan().bold());
    eprintln!(
        "  {}    {}",
        TAGLINE.bright_white(),
        format!("v{}", env!("CARGO_PKG_VERSION")).bright_black()
    );
    eprintln!(
        "  {} {}",
        "tip:".bright_black(),
        "drag a .mapgeo file or a folder onto this exe to convert it"
            .bright_black()
            .italic()
    );
    eprintln!();
}
