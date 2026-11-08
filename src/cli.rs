//! The command grammar.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use ctl_core::{EnvPrefix, OutputArgs};

use crate::color::Rgb;
use crate::effect::Effect;
use crate::report::{Brightness, Direction, Persistence, Speed};

/// Read and change MSI Mystic Light ARGB lighting.
#[derive(Debug, Parser)]
#[command(
    version,
    about = "Read and change MSI Mystic Light ARGB lighting",
    arg_required_else_help = true,
    env_prefix = "RGBCTL"
)]
pub struct Cli {
    /// Config file; defaults to the nearest .ctl/rgb.yaml, then
    /// $XDG_CONFIG_HOME/ctl/rgb.yaml.
    #[arg(long, global = true, value_hint = clap::ValueHint::FilePath)]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    #[expect(
        missing_docs,
        reason = "clap flattens these; their fields carry the help"
    )]
    pub output: OutputArgs,

    #[command(subcommand)]
    #[expect(missing_docs, reason = "each variant documents its subcommand")]
    pub command: Command,
}

/// The subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print the controller's current lighting, decoded.
    Show,
    /// Turn the LEDs off.
    Off(WriteArgs),
    /// One steady colour.
    Static(StaticArgs),
    /// One colour fading in and out.
    Breathing(BreathingArgs),
    /// Colours travelling along the strip; no colours means a generated
    /// rainbow.
    Wave(WaveArgs),
    /// The controller's own rainbow.
    Rainbow(RainbowArgs),
    /// Apply a preset from the config file, or its default preset.
    Preset(PresetArgs),
    /// List the presets in the config file.
    Presets,
    /// Print the operator contract for this version.
    Instructions,
}

/// Flags every write takes.
#[derive(Debug, Args)]
pub struct WriteArgs {
    /// Commit to the controller's flash, so the setting survives power-off.
    /// Without it the write is a preview.
    #[arg(long)]
    pub save: bool,
}

impl WriteArgs {
    /// The save flag this write sets.
    #[must_use]
    pub fn persistence(&self) -> Persistence {
        if self.save {
            Persistence::Save
        } else {
            Persistence::Preview
        }
    }
}

/// `rgbctl static`.
#[derive(Debug, Args)]
pub struct StaticArgs {
    /// #rrggbb, or red, green, blue, white, purple, cyan, yellow, orange, pink.
    pub color: Rgb,
    /// 1 to 5.
    #[arg(short, long, default_value_t)]
    pub brightness: Brightness,
    #[command(flatten)]
    #[expect(
        missing_docs,
        reason = "clap flattens these; their fields carry the help"
    )]
    pub write: WriteArgs,
}

/// `rgbctl breathing`.
#[derive(Debug, Args)]
pub struct BreathingArgs {
    /// #rrggbb, or red, green, blue, white, purple, cyan, yellow, orange, pink.
    pub color: Rgb,
    /// 1 to 5.
    #[arg(short, long, default_value_t)]
    pub brightness: Brightness,
    /// How fast it fades.
    #[arg(short, long, value_enum, default_value_t)]
    pub speed: Speed,
    #[command(flatten)]
    #[expect(
        missing_docs,
        reason = "clap flattens these; their fields carry the help"
    )]
    pub write: WriteArgs,
}

/// `rgbctl wave`.
#[derive(Debug, Args)]
pub struct WaveArgs {
    /// Up to four colours, repeated in order to fill the palette.
    #[arg(num_args = 0..=4)]
    pub colors: Vec<Rgb>,
    /// 1 to 5.
    #[arg(short, long, default_value_t)]
    pub brightness: Brightness,
    /// How fast it travels.
    #[arg(short, long, value_enum, default_value_t)]
    pub speed: Speed,
    /// Which way it travels.
    #[arg(short, long, value_enum, default_value_t)]
    pub direction: Direction,
    #[command(flatten)]
    #[expect(
        missing_docs,
        reason = "clap flattens these; their fields carry the help"
    )]
    pub write: WriteArgs,
}

/// `rgbctl rainbow`.
#[derive(Debug, Args)]
pub struct RainbowArgs {
    /// 1 to 5.
    #[arg(short, long, default_value_t)]
    pub brightness: Brightness,
    /// How fast it cycles.
    #[arg(short, long, value_enum, default_value_t)]
    pub speed: Speed,
    /// Which way it travels.
    #[arg(short, long, value_enum, default_value_t)]
    pub direction: Direction,
    #[command(flatten)]
    #[expect(
        missing_docs,
        reason = "clap flattens these; their fields carry the help"
    )]
    pub write: WriteArgs,
}

/// `rgbctl preset`.
#[derive(Debug, Args)]
pub struct PresetArgs {
    /// The preset; defaults to the file's `default`.
    pub name: Option<String>,
    #[command(flatten)]
    #[expect(
        missing_docs,
        reason = "clap flattens these; their fields carry the help"
    )]
    pub write: WriteArgs,
}

impl StaticArgs {
    /// The effect these flags ask for.
    #[must_use]
    pub fn effect(&self) -> Effect {
        Effect::Static {
            color: self.color,
            brightness: self.brightness,
        }
    }
}

impl BreathingArgs {
    /// The effect these flags ask for.
    #[must_use]
    pub fn effect(&self) -> Effect {
        Effect::Breathing {
            color: self.color,
            brightness: self.brightness,
            speed: self.speed,
        }
    }
}

impl WaveArgs {
    /// The effect these flags ask for.
    #[must_use]
    pub fn effect(&self) -> Effect {
        Effect::Wave {
            colors: self.colors.clone(),
            brightness: self.brightness,
            speed: self.speed,
            direction: self.direction,
        }
    }
}

impl RainbowArgs {
    /// The effect these flags ask for.
    #[must_use]
    pub fn effect(&self) -> Effect {
        Effect::Rainbow {
            brightness: self.brightness,
            speed: self.speed,
            direction: self.direction,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn the_grammar_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn a_write_is_a_preview_unless_save_is_given() {
        let cli = Cli::try_parse_from(["rgbctl", "static", "red"]).unwrap();
        let Command::Static(args) = cli.command else {
            panic!("static parses as Static");
        };
        assert_eq!(args.write.persistence(), Persistence::Preview);
        let cli = Cli::try_parse_from(["rgbctl", "off", "--save"]).unwrap();
        let Command::Off(args) = cli.command else {
            panic!("off parses as Off");
        };
        assert_eq!(args.persistence(), Persistence::Save);
    }

    #[test]
    fn a_wave_takes_at_most_four_colours() {
        assert!(Cli::try_parse_from(["rgbctl", "wave", "red", "blue", "red", "blue"]).is_ok());
        assert!(
            Cli::try_parse_from(["rgbctl", "wave", "red", "blue", "red", "blue", "red"]).is_err()
        );
    }

    #[test]
    fn brightness_outside_one_to_five_is_refused() {
        assert!(Cli::try_parse_from(["rgbctl", "rainbow", "-b", "0"]).is_err());
        assert!(Cli::try_parse_from(["rgbctl", "rainbow", "-b", "6"]).is_err());
        assert!(Cli::try_parse_from(["rgbctl", "rainbow", "-b", "1"]).is_ok());
    }

    #[test]
    fn flags_default_to_the_reference_values() {
        let cli = Cli::try_parse_from(["rgbctl", "wave"]).unwrap();
        let Command::Wave(args) = cli.command else {
            panic!("wave parses as Wave");
        };
        assert_eq!(
            args.effect(),
            Effect::Wave {
                colors: vec![],
                brightness: Brightness::MAX,
                speed: Speed::Medium,
                direction: Direction::Right,
            }
        );
    }
}
