//! What each command returns: one serializable value, with its human
//! presentation. JSON serializes the value itself.

use std::collections::BTreeMap;

use ctl_core::{Document, Fields, Present, Section, Text};
use serde::Serialize;

use crate::effect::Effect;
use crate::report::{FeatureReport, Lighting, Persistence};

/// A command's result.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Outcome {
    /// `rgbctl show`.
    Shown {
        /// The hidraw node read.
        device: String,
        /// What it holds.
        state: State,
    },
    /// A write that read back as written.
    Applied {
        /// The hidraw node written.
        device: String,
        /// The effect written.
        effect: Effect,
        /// Preview or save.
        persistence: Persistence,
        /// The state before the write.
        before: State,
        /// The state read back after it.
        after: State,
    },
    /// `rgbctl presets`.
    Presets {
        /// The config file, or `None` when none exists.
        path: Option<String>,
        /// The preset applied when `rgbctl preset` is given no name.
        default: Option<String>,
        /// Presets by name.
        presets: BTreeMap<String, Effect>,
    },
    /// `rgbctl instructions`.
    Instructions {
        /// The operator contract.
        markdown: String,
    },
}

/// A report as read, decoded when it decodes.
#[derive(Debug, Serialize)]
pub struct State {
    /// Bytes 273 to 289 in hex.
    pub bytes: String,
    /// The decoded lighting, or `None` when a byte holds a value no known
    /// effect uses.
    pub lighting: Option<Lighting>,
    /// Why the bytes did not decode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

impl From<&FeatureReport> for State {
    fn from(report: &FeatureReport) -> Self {
        let (lighting, problem) = match report.lighting() {
            Ok(lighting) => (Some(lighting), None),
            Err(error) => (None, Some(error.to_string())),
        };
        Self {
            bytes: report.owned_hex(),
            lighting,
            problem,
        }
    }
}

impl Present for Outcome {
    fn present(&self) -> Document {
        match self {
            Self::Shown { device, state } => {
                Document::new().fields(state_fields(state).row("device", Text::new().value(device)))
            }
            Self::Applied {
                device,
                persistence,
                before,
                after,
                ..
            } => {
                let verb = match persistence {
                    Persistence::Preview => "previewed",
                    Persistence::Save => "saved",
                };
                Document::new()
                    .paragraph(Text::new().success(verb).then("  ").value(device))
                    .section(Section::new(
                        "before",
                        Document::new().fields(state_fields(before)),
                    ))
                    .section(Section::new(
                        "after",
                        Document::new().fields(state_fields(after)),
                    ))
            }
            Self::Presets {
                path,
                default,
                presets,
            } => {
                let mut fields = Fields::new().row(
                    "file",
                    path.as_deref().map_or_else(
                        || Text::new().muted("none; built-in defaults"),
                        |path| Text::new().value(path),
                    ),
                );
                if let Some(default) = default {
                    fields = fields.row("default", Text::new().id(default));
                }
                for (name, effect) in presets {
                    fields = fields.row(name.as_str(), Text::plain(describe(effect)));
                }
                Document::new().fields(fields)
            }
            Self::Instructions { markdown } => Document::new().verbatim(markdown.clone()),
        }
    }
}

fn state_fields(state: &State) -> Fields {
    let fields = Fields::new();
    let fields = match (&state.lighting, &state.problem) {
        (Some(lighting), _) => {
            let style = lighting.style;
            fields
                .row("mode", Text::new().value(name(&lighting.mode)))
                .row(
                    "palette",
                    Text::plain(
                        lighting
                            .palette
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                )
                .row("brightness", Text::plain(style.brightness.to_string()))
                .row("speed", Text::plain(name(&style.speed)))
                .row("direction", Text::plain(name(&style.direction)))
                .row("colours from", Text::plain(name(&style.source)))
                .row("animate", Text::plain(style.animate.to_string()))
        }
        (None, problem) => fields.row(
            "problem",
            Text::new().warning(problem.as_deref().unwrap_or("does not decode")),
        ),
    };
    fields.row("bytes 273-289", Text::new().muted(&state.bytes))
}

/// The snake_case name serde gives a unit variant.
fn name(value: &impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn describe(effect: &Effect) -> String {
    match effect {
        Effect::Off => "off".to_owned(),
        Effect::Static { color, brightness } => format!("static {color}, brightness {brightness}"),
        Effect::Breathing {
            color,
            brightness,
            speed,
        } => format!(
            "breathing {color}, brightness {brightness}, {}",
            name(speed)
        ),
        Effect::Wave {
            colors,
            brightness,
            speed,
            direction,
        } => {
            let colors = if colors.is_empty() {
                "generated colours".to_owned()
            } else {
                colors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            format!(
                "wave {colors}, brightness {brightness}, {}, {}",
                name(speed),
                name(direction)
            )
        }
        Effect::Rainbow {
            brightness,
            speed,
            direction,
        } => format!(
            "rainbow, brightness {brightness}, {}, {}",
            name(speed),
            name(direction)
        ),
    }
}
