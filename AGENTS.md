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
3. **Report model.**
   Report `0x50` is 290 bytes, the id included.
   rgbctl owns bytes 273 to 287 (mode, four-colour palette, sub-parameter, packed style byte) and byte 289 (save flag).
   Every other byte is opaque and is written back exactly as read.
   The crate carries no template report.
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

## Rules

- ctl's shared rules apply: always a pull request, never `--no-verify`, ctl-core owns presentation, every flag has an `RGBCTL_` variable, and nothing local leaks into a tracked file.
- The pull request's CI verifies. Do not build or test before a push.
- A behaviour change moves code, tests, `src/instructions.md`, `skills/rgbctl/SKILL.md`, this file and `docs/` together (the ctl bundle contract).
- Hardware access is Linux-only. Code that needs the device sits behind `cfg(target_os = "linux")`;
  the report model and its tests are platform-independent.
