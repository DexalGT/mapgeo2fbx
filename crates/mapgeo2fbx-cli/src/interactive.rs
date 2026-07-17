use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;

use crate::args::{Cli, LogLevel};

/// Runs the numbered menu loop shown on a pure double-click (no args). Converges on the same
/// `crate::run` used by the flagged and drag-drop paths so behavior stays consistent.
pub fn run() -> Result<()> {
    crate::banner::print();

    loop {
        let action = prompt_action()?;
        match action {
            Action::Convert => {
                if let Some(path) =
                    prompt_path("Drop a .mapgeo file or folder here (or paste a path)")?
                {
                    let cli = baseline_cli(path);
                    if let Err(e) = crate::run(&cli) {
                        eprintln!("{} {e:#}", "error:".bright_red().bold());
                    }
                }
            }
            Action::InfoOnly => {
                if let Some(path) =
                    prompt_path("Drop a .mapgeo file here to inspect (or paste a path)")?
                {
                    let mut cli = baseline_cli(path);
                    cli.info_only = true;
                    cli.verbose = true;
                    if let Err(e) = crate::run(&cli) {
                        eprintln!("{} {e:#}", "error:".bright_red().bold());
                    }
                }
            }
            Action::Quit => break,
        }

        if !prompt_yes_no("Do another?", true)? {
            break;
        }
    }

    eprintln!("\n  {}\n", "bye!".bright_cyan());
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Convert,
    InfoOnly,
    Quit,
}

const MENU: &[(Action, &str, &str)] = &[
    (
        Action::Convert,
        "Convert a .mapgeo (or folder)",
        "writes .fbx next to each source",
    ),
    (
        Action::InfoOnly,
        "Show info only",
        "inspect a .mapgeo without converting",
    ),
    (Action::Quit, "Quit", ""),
];

fn prompt_action() -> Result<Action> {
    eprintln!("  {}", "What do you want to do?".bright_white().bold());
    eprintln!();
    for (i, (_, label, hint)) in MENU.iter().enumerate() {
        if hint.is_empty() {
            eprintln!("    [{}]  {}", i + 1, label);
        } else {
            eprintln!(
                "    [{}]  {} {}",
                i + 1,
                label,
                format!("— {hint}").bright_black()
            );
        }
    }
    eprintln!();

    loop {
        let raw = read_line(&format!("  choice (1-{}): ", MENU.len()))?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if matches!(lower.as_str(), "q" | "quit" | "exit") {
            return Ok(Action::Quit);
        }
        if let Ok(n) = trimmed.parse::<usize>() {
            if n >= 1 && n <= MENU.len() {
                return Ok(MENU[n - 1].0);
            }
        }
        eprintln!(
            "  not a valid choice — try 1-{} (or 'q' to quit).",
            MENU.len()
        );
    }
}

fn prompt_path(label: &str) -> Result<Option<PathBuf>> {
    eprintln!();
    eprintln!("  -> {label}");
    eprintln!("     (empty to cancel; quotes auto-stripped)");

    loop {
        let raw = read_line("  path: ")?;
        let cleaned = strip_path_quotes(raw.trim());
        if cleaned.is_empty() {
            return Ok(None);
        }
        let path = PathBuf::from(&cleaned);
        if path.exists() {
            return Ok(Some(path));
        }
        eprintln!("  not found: {cleaned} (try again or empty to cancel)");
    }
}

fn prompt_yes_no(label: &str, default_yes: bool) -> Result<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    loop {
        let raw = read_line(&format!("  {label} {hint} "))?;
        let trimmed = raw.trim().to_lowercase();
        if trimmed.is_empty() {
            return Ok(default_yes);
        }
        match trimmed.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => eprintln!("  answer 'y' or 'n'."),
        }
    }
}

fn read_line(prompt: &str) -> Result<String> {
    let mut stdout = io::stderr();
    write!(stdout, "{prompt}")?;
    stdout.flush()?;
    let mut buf = String::new();
    io::stdin().lock().read_line(&mut buf)?;
    while buf.ends_with('\n') || buf.ends_with('\r') {
        buf.pop();
    }
    Ok(buf)
}

fn strip_path_quotes(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

fn baseline_cli(input: PathBuf) -> Cli {
    Cli {
        input,
        output: None,
        info_only: false,
        verbose: false,
        json: false,
        log_level: LogLevel::Normal,
        no_pause: true,
    }
}
