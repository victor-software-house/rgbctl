//! Each command, from parsed arguments to its [`Outcome`].

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};

use crate::cli::{Cli, Command, PresetArgs, WriteArgs};
use crate::config::{Loaded, Search, user_config_dir};
use crate::effect::Effect;
use crate::outcome::Outcome;
use crate::report::Persistence;

/// The operator contract, `rgbctl instructions`.
pub const INSTRUCTIONS: &str = include_str!("instructions.md");

/// Run one command.
///
/// # Errors
///
/// A config file that does not load, a preset that does not exist, or any
/// failure to reach, read, write, or verify the controller.
pub fn execute(cli: Cli) -> Result<Outcome> {
    match cli.command {
        Command::Show => device::show(),
        Command::Off(write) => apply(&Effect::Off, &write),
        Command::Static(args) => apply(&args.effect(), &args.write),
        Command::Breathing(args) => apply(&args.effect(), &args.write),
        Command::Wave(args) => apply(&args.effect(), &args.write),
        Command::Rainbow(args) => apply(&args.effect(), &args.write),
        Command::Preset(args) => preset(&load_config(cli.config)?, &args),
        Command::Presets => {
            let Loaded { path, config } = load_config(cli.config)?;
            Ok(Outcome::Presets {
                path: path.map(|path| path.display().to_string()),
                default: config.default,
                presets: config.presets,
            })
        }
        Command::Instructions => Ok(Outcome::Instructions {
            markdown: INSTRUCTIONS.to_owned(),
        }),
    }
}

/// Only the commands that read presets look for a config file, so the others
/// run even from a working directory that no longer exists.
fn load_config(explicit: Option<PathBuf>) -> Result<Loaded> {
    let search = Search {
        explicit,
        working_dir: env::current_dir().context("reading the working directory")?,
        user_config_dir: user_config_dir(
            env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            env::var_os("HOME").map(PathBuf::from),
        ),
    };
    Ok(search.load()?)
}

fn apply(effect: &Effect, write: &WriteArgs) -> Result<Outcome> {
    effect.check()?;
    device::apply(effect, write.persistence())
}

fn preset(loaded: &Loaded, args: &PresetArgs) -> Result<Outcome> {
    let file = || {
        loaded.path.as_ref().map_or_else(
            || "no config file".to_owned(),
            |path| path.display().to_string(),
        )
    };
    let name = args
        .name
        .as_ref()
        .or(loaded.config.default.as_ref())
        .ok_or_else(|| anyhow!("name a preset, or set `default` in {}", file()))?;
    let effect = loaded
        .config
        .presets
        .get(name)
        .ok_or_else(|| anyhow!("no preset `{name}` in {}", file()))?;
    apply(effect, &args.write)
}

#[cfg(target_os = "linux")]
mod device {
    use std::path::Path;

    use anyhow::Result;

    use super::Persistence;
    use crate::cycle::{plan, verify};
    use crate::effect::Effect;
    use crate::hidraw::Controller;
    use crate::outcome::{Outcome, State};

    pub(super) fn show() -> Result<Outcome> {
        let (controller, report) = Controller::open(Path::new("/sys"))?;
        Ok(Outcome::Shown {
            device: controller.path().display().to_string(),
            state: State::from(&report),
        })
    }

    /// Read, change the owned bytes, write, read back, compare.
    pub(super) fn apply(effect: &Effect, persistence: Persistence) -> Result<Outcome> {
        let (controller, before) = Controller::open(Path::new("/sys"))?;
        let written = plan(&before, effect, persistence);
        controller.write(&written)?;
        let after = controller.read()?;
        verify(&written, &after)?;
        Ok(Outcome::Applied {
            device: controller.path().display().to_string(),
            effect: effect.clone(),
            persistence,
            before: State::from(&before),
            after: State::from(&after),
        })
    }
}

#[cfg(not(target_os = "linux"))]
mod device {
    use anyhow::{Result, bail};

    use super::Persistence;
    use crate::effect::Effect;
    use crate::outcome::Outcome;

    const LINUX_ONLY: &str = "rgbctl reaches the controller through Linux hidraw only";

    pub(super) fn show() -> Result<Outcome> {
        bail!(LINUX_ONLY)
    }

    pub(super) fn apply(_: &Effect, _: Persistence) -> Result<Outcome> {
        bail!(LINUX_ONLY)
    }
}
