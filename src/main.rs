mod captor;
mod controller;
mod display;
mod normaliser;
mod settings;
mod transform;

use std::error::Error;
use std::path::PathBuf;

use clap::Parser;
use crossterm::terminal;

use crate::controller::Controller;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// path to the YAML configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let settings = settings::Settings::load(&args.config)?;

    let (terminal_width, terminal_height) = terminal::size()?;

    let controller = Controller::new(
        terminal_height,
        terminal_width,
        settings.sample_window,
        settings.sample_rate,
        settings.framerate,
        settings.min_frequency,
        settings.max_frequency,
        settings.merger,
        controller::Mode::Monolithic,
    );

    controller.run();

    Ok(())
}
