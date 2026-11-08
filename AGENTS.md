# rgbctl

Linux-only ctl CLI for the MSI Mystic Light controller (USB `0db0:0076`), built on ctl-core.
This file is the development contract.
The operator contract will be `rgbctl instructions` and `skills/rgbctl/SKILL.md`.

## Decisions

1. **Device lookup.**
   The controller is found by walking `/sys/class/hidraw/*/device/uevent`
   for `HID_ID=0003:00000DB0:00000076`.
   Never hard-code a `/dev/hidrawN` number; it changes between boots.
2. **HID access.**
   Plain `HIDIOCGFEATURE` and `HIDIOCSFEATURE` ioctls on the hidraw node, through `rustix`.
   No hidapi, libusb or libudev, so the release is a static musl binary that runs on any distribution, NixOS included.
   The ioctl calls live in one small module;
   each carries an item-level `#[expect(unsafe_code, reason = "…")]` and a `// SAFETY:` comment.
   `unsafe_code` stays `deny` everywhere else.
   rgbctl sends no output report: after the `0x01` `0x82` queries, or the wave write 5 s later (a later clean-state wave did not repeat it), a connected cooler entered a state only a power cut cleared (`docs/hardware-test-2026-11-08.md`).
3. **Report model.**
   Report `0x50` is 290 bytes, the id included.
   rgbctl owns bytes 273 to 287 (mode, four-colour palette, sub-parameter, packed style byte) and byte 289 (save flag).
   Every other byte is opaque and is written back exactly as read.
   The crate carries no template report.
   The owned bytes are the last of 18 sixteen-byte zone records, the one that applies to all zones.
   `docs/protocol.md` records the record layout, what each field and mode byte means, and the source of each fact; update it with every new finding.
4. **Write cycle.**
   Read, decode, change the owned bytes, write, read back, compare.
   A read-back that differs from what was written outside the save flag is an error.
5. **Volatile by default.**
   A write sets the save flag only with `--save`.
   Tests on real hardware are volatile and run with someone watching the lights.
6. **Config.**
   `--config` (or `RGBCTL_CONFIG`) names the file.
   Otherwise rgbctl uses the nearest `.ctl/rgb.yaml` walking up from the working directory,
   then `$XDG_CONFIG_HOME/ctl/rgb.yaml` (`~/.config/ctl/rgb.yaml`).
   No file means built-in defaults.
   The file holds named presets and the name of the default one.
7. **No daemon, no boot service.**
   The controller keeps a saved setting through power cycles.
   Effects that need a running process, such as colour by CPU temperature, are out of scope.
8. **Permissions** come from a udev rule granting a group access to the hidraw node (see `README.md`).
   The binary never escalates.
9. **Releases are GitHub Release binaries only**, static musl builds for `x86_64` and `aarch64`.
   The crate is not published to crates.io (`publish = false`): it is a hardware tool for one controller, and its consumers install prebuilt binaries.

## Layout

- `src/report.rs` models report `0x50`; `src/effect.rs` maps each effect to its bytes; `src/cycle.rs` plans a write and verifies the read-back.
  These three and `src/config.rs` build and test on any platform.
- `src/hidraw.rs` (Linux only) finds the node and holds the two ioctls.
- `src/run.rs` runs each command; `src/outcome.rs` is what a command returns, as JSON and as a human document.
- `src/instructions.md` (`rgbctl instructions`) and `skills/rgbctl/SKILL.md` are renders of `.ctl/operator/`: one shared `contract.md.jinja`, its command table from ctl-core's `Surface` over the Clap grammar, and the skill's `version` from `CARGO_PKG_VERSION`.
  `tests/it/docs.rs` byte-compares both committed files with the render; edit the templates, never the renders, and regenerate with `UPDATE_OPERATOR_DOCS=1 cargo test docs` in CI's toolchain.
- `tests/fixtures/report-0x50-off.txt` is a read-back of a board saved as off, in the probe's `offset: bytes` format.
- `docs/protocol.md` holds what is known about report `0x50`, its modes, and the connected devices, each fact with its source.
- `docs/hardware-test-2026-11-08.md` records every write of the first hardware test and what was seen.
- `PORTING.md` maps the reference script to this crate and lists every deliberate difference.
- The repository setup comes from the shared Copier template (project type `rust-crate`); `.copier-answers.yml` records its tag.
  Local edits that survive updates: `Cargo.toml` sets `unsafe_code = "deny"` instead of `forbid`.

## Rules

- ctl's shared rules apply: always a pull request, never `--no-verify`, ctl-core owns presentation, every flag has an `RGBCTL_` variable, and nothing local leaks into a tracked file.
- The pull request's CI verifies. Do not build or test before a push.
- A behaviour change moves code, tests, `src/instructions.md`, `skills/rgbctl/SKILL.md`, this file and `docs/` together (the ctl bundle contract).
- Hardware access is Linux-only. Code that needs the device sits behind `cfg(target_os = "linux")`;
  the report model and its tests are platform-independent.
