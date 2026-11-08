//! Starting the rgbctl binary for a test.

use std::process::Command;

/// The colour variables an agent shell sets, removed from every child so the
/// suite passes there; a colour test sets its own on top.
const COLOR_VARIABLES: [&str; 5] = [
    "CLICOLOR",
    "CLICOLOR_FORCE",
    "FORCE_COLOR",
    "NO_COLOR",
    "COLORTERM",
];

/// The binary under test, with a clean colour environment and no `RGBCTL_`
/// settings inherited from the caller.
pub(crate) fn rgbctl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rgbctl"));
    for name in COLOR_VARIABLES {
        command.env_remove(name);
    }
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("RGBCTL_") {
            command.env_remove(name);
        }
    }
    command
}
