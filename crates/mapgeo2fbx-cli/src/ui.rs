use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Live,
    Silent,
}

impl Mode {
    pub fn from_json(json: bool) -> Self {
        if json {
            Mode::Silent
        } else {
            Mode::Live
        }
    }
}

#[derive(Clone)]
pub struct UiReporter {
    bar: Option<ProgressBar>,
}

impl UiReporter {
    pub fn new(mode: Mode, total: u64) -> Self {
        let bar = match mode {
            Mode::Silent => None,
            Mode::Live => {
                let pb = ProgressBar::new(total);
                pb.set_style(
                    ProgressStyle::with_template("  [{bar:30.cyan/black}] {pos}/{len} {msg}")
                        .expect("hard-coded progress style is valid")
                        .progress_chars("█▉▊▋▌▍▎  "),
                );
                Some(pb)
            }
        };
        Self { bar }
    }

    pub fn tick(&self) {
        if let Some(bar) = &self.bar {
            bar.inc(1);
        }
    }

    pub fn converted(&self, name: &str) {
        let line = format!("  {} {}", "✓".bright_green().bold(), name);
        match &self.bar {
            Some(bar) => bar.println(line),
            None => eprintln!("{line}"),
        }
    }

    pub fn failed(&self, name: &str, error: &str) {
        let line = format!(
            "  {} {} — {}",
            "✗".bright_red().bold(),
            name,
            error.bright_black()
        );
        match &self.bar {
            Some(bar) => bar.println(line),
            None => eprintln!("{line}"),
        }
    }

    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }
}
