mod captor;
mod controller;
mod display;
mod normaliser;
mod transformer;

use std::error::Error;

use clap::Parser;
use crossterm::terminal;

use crate::controller::Controller;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    ///number of samples to analyse per iteration
    #[arg(short, long, default_value_t = 8192)]
    sample_window: usize,

    ///minimum frequency to display
    #[arg(long = "min", default_value_t = 4096)]
    min_frequency: usize,

    ///maximum frequency to display
    #[arg(long = "max", default_value_t = 1)]
    max_frequency: usize,

    ///target framerate
    #[arg(short, long, default_value_t = 20)]
    framerate: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let (terminal_width, terminal_height) = terminal::size()?;

    let controller = Controller::new(
        terminal_height,
        terminal_width,
        args.sample_window,
        args.framerate,
        args.min_frequency,
        args.max_frequency,
        controller::Mode::Monolithic,
    );

    controller.run();

    Ok(())
}
