//! The binary's machine output.

use ctl_core::indoc;

use crate::common::rgbctl;

#[test]
fn json_output_carries_no_colour_codes_even_when_colour_is_forced() {
    let output = rgbctl()
        .args(["instructions", "--format", "json"])
        .env("CLICOLOR_FORCE", "1")
        .env("FORCE_COLOR", "3")
        .env("COLORTERM", "truecolor")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(!output.stdout.contains(&0x1b), "ESC in JSON output");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value.to_string().contains("rgbctl agent instructions"));
}

#[test]
fn a_parse_error_in_json_mode_carries_no_colour_codes() {
    let output = rgbctl()
        .args(["static", "not-a-colour", "--format", "json"])
        .env("CLICOLOR_FORCE", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        !output.stdout.contains(&0x1b) && !output.stderr.contains(&0x1b),
        "{output:?}"
    );
}

#[test]
fn presets_lists_the_file_given_by_the_environment() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rgb.yaml");
    std::fs::write(
        &path,
        indoc! {"
            default: dark
            presets:
              dark:
                effect: off
        "},
    )
    .unwrap();
    let output = rgbctl()
        .args(["presets", "--format", "json"])
        .env("RGBCTL_CONFIG", &path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["default"], "dark", "{json}");
    assert_eq!(json["presets"]["dark"]["effect"], "off", "{json}");
}

#[test]
fn a_preset_that_does_not_exist_is_refused_before_any_device_access() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rgb.yaml");
    std::fs::write(&path, "presets: {}\n").unwrap();
    let output = rgbctl()
        .args(["preset", "nope", "--config"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(text.contains("no preset `nope`"), "{text}");
}
