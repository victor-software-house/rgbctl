//! The rgbctl binary.

use ctl_core::prelude::{App, ExitCode};
use rgbctl::cli::Cli;

fn main() -> ExitCode {
    App::<Cli>::new("rgbctl")
        .view(|cli| cli.output.view())
        .run(rgbctl::run::execute)
}
