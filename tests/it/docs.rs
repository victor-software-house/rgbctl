//! The bundled skill and `rgbctl instructions` state the same contract.

/// The text after a document's first heading line.
fn body(document: &str) -> &str {
    let heading = document.find("\n# ").map_or(0, |at| at + 1);
    let rest = &document[heading..];
    rest.split_once('\n').map_or("", |(_, body)| body)
}

#[test]
fn the_skill_body_is_the_instructions_body() {
    let skill = include_str!("../../skills/rgbctl/SKILL.md");
    let instructions = include_str!("../../src/instructions.md");
    assert_eq!(body(skill), body(instructions));
}

#[test]
fn the_skill_version_is_the_crate_version() {
    let skill = include_str!("../../skills/rgbctl/SKILL.md");
    let version = format!("\nversion: {}\n", env!("CARGO_PKG_VERSION"));
    assert!(skill.contains(&version), "SKILL.md must say {version:?}");
}
