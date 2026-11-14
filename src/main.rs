//! The rgbctl binary.

use ctl_core::prelude::{App, ExitCode};
use rgbctl::cli::Cli;
use rgbctl::run;

fn main() -> ExitCode {
    App::<Cli>::new("rgbctl")
        .view(|cli| cli.output.view())
        .run(run::execute)
}
