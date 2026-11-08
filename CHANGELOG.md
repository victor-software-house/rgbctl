# Changelog

## rgbctl 0.0.1

### Added

- First release. `rgbctl` reads and changes the ARGB lighting of the MSI Mystic Light controller (USB `0db0:0076`) over Linux hidraw, with no vendor software.
It shows the decoded state and applies off, static, breathing, wave, and rainbow, plus presets from `.ctl/rgb.yaml`.
Every write is a preview unless it is given `--save`, and every write is read back and verified.
Releases are static musl binaries for x86_64 and aarch64 Linux ([#2](https://github.com/victor-software-house/rgbctl/pull/2)).

