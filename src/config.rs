//! `.ctl/rgb.yaml`: named presets and the one applied by default.
//!
//! ```yaml
//! default: ember
//! presets:
//!   ember:
//!     effect: static
//!     color: "#ff3c00"
//!     brightness: 2
//!   party:
//!     effect: wave
//!     colors: [red, blue]
//!     speed: fast
//!     direction: left
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ctl_core::input::{Input, InputError};
use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::effect::Effect;

/// The file's path below a directory that holds `.ctl/`.
pub const REPO_FILE: &str = ".ctl/rgb.yaml";
/// The file's path below the user's config directory.
pub const USER_FILE: &str = "ctl/rgb.yaml";

/// The whole config file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The preset `rgbctl preset` applies when given no name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(custom(names_a_preset(&self.presets)))]
    pub default: Option<String>,
    /// Presets by name.
    #[serde(default)]
    #[garde(custom(presets_fit))]
    pub presets: BTreeMap<String, Effect>,
}

fn names_a_preset(
    presets: &BTreeMap<String, Effect>,
) -> impl FnOnce(&Option<String>, &()) -> garde::Result + '_ {
    move |default, ()| match default {
        Some(name) if !presets.contains_key(name) => Err(garde::Error::new(format!(
            "`{name}` is not one of the presets; add it under presets or name another"
        ))),
        Some(_) | None => Ok(()),
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "garde passes the validation context by reference"
)]
fn presets_fit(presets: &BTreeMap<String, Effect>, _: &()) -> garde::Result {
    presets
        .iter()
        .find_map(|(name, effect)| {
            effect
                .check()
                .err()
                .map(|error| garde::Error::new(format!("preset `{name}`: {error}")))
        })
        .map_or(Ok(()), Err)
}

/// The config in force, and the file it came from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Loaded {
    /// The file read, or `None` when no file exists and the defaults apply.
    pub path: Option<PathBuf>,
    /// Its contents.
    pub config: Config,
}

/// Where rgbctl looks for its config, from the process environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Search {
    /// `--config`, or `RGBCTL_CONFIG`.
    pub explicit: Option<PathBuf>,
    /// The working directory, whose ancestors may hold `.ctl/rgb.yaml`.
    pub working_dir: PathBuf,
    /// `$XDG_CONFIG_HOME`, or `$HOME/.config`.
    pub user_config_dir: Option<PathBuf>,
}

impl Search {
    /// The file to read: the explicit path, else the nearest
    /// `.ctl/rgb.yaml` above the working directory, else
    /// `<user config dir>/ctl/rgb.yaml`; `None` when none exists.
    #[must_use]
    pub fn locate(&self) -> Option<PathBuf> {
        if let Some(path) = &self.explicit {
            return Some(path.clone());
        }
        self.working_dir
            .ancestors()
            .map(|dir| dir.join(REPO_FILE))
            .chain(self.user_config_dir.iter().map(|dir| dir.join(USER_FILE)))
            .find(|path| path.is_file())
    }

    /// Read and validate the file [`Search::locate`] finds.
    ///
    /// # Errors
    ///
    /// [`InputError`] when the file cannot be read, is not valid YAML, has an
    /// unknown field, or names a default that is not a preset. An explicit
    /// path that does not exist is an error; a missing default file is not.
    pub fn load(&self) -> Result<Loaded, InputError> {
        match self.locate() {
            Some(path) => Ok(Loaded {
                config: load(&path)?,
                path: Some(path),
            }),
            None => Ok(Loaded::default()),
        }
    }
}

/// Read and validate one config file.
///
/// # Errors
///
/// As [`Search::load`].
pub fn load(path: &Path) -> Result<Config, InputError> {
    Input::open(path)?.load::<Config>()
}

/// `$XDG_CONFIG_HOME` when set to an absolute path, else `$HOME/.config`.
#[must_use]
pub fn user_config_dir(xdg_config_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    xdg_config_home
        .filter(|dir| dir.is_absolute())
        .or_else(|| home.map(|home| home.join(".config")))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ctl_core::indoc;

    use super::*;
    use crate::color::Rgb;
    use crate::report::{Brightness, Direction, Speed};

    fn parse(text: &str) -> Result<Config, InputError> {
        Input::new("rgb.yaml", text).load::<Config>()
    }

    #[test]
    fn the_documented_example_loads() {
        let config = parse(indoc! {r##"
            default: ember
            presets:
              ember:
                effect: static
                color: "#ff3c00"
                brightness: 2
              party:
                effect: wave
                colors: [red, blue]
                speed: fast
                direction: left
        "##})
        .unwrap();
        assert_eq!(config.default.as_deref(), Some("ember"));
        assert_eq!(
            config.presets["ember"],
            Effect::Static {
                color: Rgb::new(255, 60, 0),
                brightness: Brightness::try_from(2).unwrap(),
            }
        );
        assert_eq!(
            config.presets["party"],
            Effect::Wave {
                colors: vec![Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)],
                brightness: Brightness::MAX,
                speed: Speed::Fast,
                direction: Direction::Left,
            }
        );
    }

    #[test]
    fn an_unquoted_off_is_the_off_effect() {
        let config = parse("presets: {dark: {effect: off}}").unwrap();
        assert_eq!(config.presets["dark"], Effect::Off);
    }

    #[test]
    fn an_empty_file_is_the_defaults() {
        assert_eq!(parse("{}").unwrap(), Config::default());
    }

    #[test]
    fn a_default_that_is_not_a_preset_is_refused() {
        let error = parse(indoc! {"
            default: nope
            presets:
              dark:
                effect: off
        "})
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("`nope` is not one of the presets"),
            "{error}"
        );
    }

    #[test]
    fn an_unknown_field_in_a_preset_is_refused() {
        let error = parse(indoc! {"
            presets:
              a:
                effect: static
                color: red
                colour: red
        "})
        .unwrap_err();
        assert!(error.to_string().contains("colour"), "{error}");
    }

    #[test]
    fn a_missing_colour_is_refused() {
        assert!(parse("presets: {a: {effect: breathing}}").is_err());
    }

    #[test]
    fn brightness_outside_one_to_five_is_refused() {
        assert!(parse("presets: {a: {effect: rainbow, brightness: 6}}").is_err());
    }

    #[test]
    fn a_wave_of_five_colours_is_refused() {
        let error =
            parse("presets: {a: {effect: wave, colors: [red, red, red, red, red]}}").unwrap_err();
        assert!(error.to_string().contains("at most 4 colours"), "{error}");
    }

    #[test]
    fn the_nearest_repo_file_wins_over_the_user_file() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("a/b");
        fs::create_dir_all(nested.join(".ctl")).unwrap();
        fs::create_dir_all(root.path().join(".ctl")).unwrap();
        fs::create_dir_all(root.path().join("xdg/ctl")).unwrap();
        for dir in [root.path(), &nested] {
            fs::write(dir.join(REPO_FILE), "{}").unwrap();
        }
        fs::write(root.path().join("xdg").join(USER_FILE), "{}").unwrap();
        let search = Search {
            explicit: None,
            working_dir: nested.join("deeper"),
            user_config_dir: Some(root.path().join("xdg")),
        };
        assert_eq!(search.locate(), Some(nested.join(REPO_FILE)));
    }

    #[test]
    fn the_user_file_applies_when_no_repo_file_exists() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("xdg/ctl")).unwrap();
        fs::write(root.path().join("xdg").join(USER_FILE), "{}").unwrap();
        let search = Search {
            explicit: None,
            working_dir: root.path().join("work"),
            user_config_dir: Some(root.path().join("xdg")),
        };
        assert_eq!(
            search.locate(),
            Some(root.path().join("xdg").join(USER_FILE))
        );
    }

    #[test]
    fn no_file_means_the_defaults_and_an_explicit_missing_file_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let mut search = Search {
            explicit: None,
            working_dir: root.path().to_path_buf(),
            user_config_dir: None,
        };
        assert_eq!(search.load().unwrap(), Loaded::default());
        search.explicit = Some(root.path().join("missing.yaml"));
        assert!(search.load().is_err());
    }

    #[test]
    fn user_config_dir_prefers_an_absolute_xdg_config_home() {
        let home = Some(PathBuf::from("/home/u"));
        assert_eq!(
            user_config_dir(Some(PathBuf::from("/xdg")), home.clone()),
            Some(PathBuf::from("/xdg"))
        );
        assert_eq!(
            user_config_dir(Some(PathBuf::from("relative")), home.clone()),
            Some(PathBuf::from("/home/u/.config"))
        );
        assert_eq!(user_config_dir(None, None), None);
    }
}
