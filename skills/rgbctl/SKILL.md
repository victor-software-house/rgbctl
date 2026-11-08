---
name: rgbctl
description: >-
  Operate rgbctl, the MSI Mystic Light ARGB controller CLI for Linux: show the
  current lighting, turn it off, set static, breathing, wave or rainbow
  effects with colours, brightness, speed and direction, apply presets from
  .ctl/rgb.yaml, and save to the controller's flash. Use when the user
  mentions rgbctl, Mystic Light, motherboard ARGB or RGB lighting, case fan
  or pump LEDs on an MSI board, or USB 0db0:0076.
license: MIT
version: 0.0.0
compatibility: Requires an rgbctl binary on Linux and read-write access to the controller's hidraw node.
---

# rgbctl

`rgbctl` reads and changes the lighting of an MSI Mystic Light controller (USB `0db0:0076`) on Linux.
It finds the controller's `/dev/hidrawN` node by vendor and product id; the node number is never fixed.

## Commands

| Command | Purpose |
|:--|:--|
| `rgbctl show` | Print the current lighting, decoded, with bytes 273 to 289 in hex |
| `rgbctl off` | LEDs off |
| `rgbctl static <COLOR> [-b 1-5]` | One steady colour |
| `rgbctl breathing <COLOR> [-b 1-5] [-s slow\|medium\|fast]` | One colour fading in and out |
| `rgbctl wave [COLOR…] [-b] [-s] [-d right\|left]` | Up to four colours travelling; none means a generated rainbow |
| `rgbctl rainbow [-b] [-s] [-d]` | The controller's own rainbow |
| `rgbctl preset [NAME]` | Apply a preset from the config file, or its `default` |
| `rgbctl presets` | List the presets and the file they came from |
| `rgbctl instructions` | Print this text |

A colour is `#rrggbb`, `rrggbb`, or one of red, green, blue, white, purple, cyan, yellow, orange, pink.
Defaults: brightness 5, speed medium, direction right.

## Writes

- Every write is a preview the controller forgets at power-off.
  `--save` commits it to the controller's flash, which then survives power cycles.
- Do not pass `--save` unless the operator asked for that setting to persist.
- Every write reads report `0x50`, changes only bytes 273 to 287 and the save flag at 289, writes it, and reads it back.
  A read-back that differs is an error naming the offsets.

## Config

`--config` (or `RGBCTL_CONFIG`) names the file.
Otherwise rgbctl reads the nearest `.ctl/rgb.yaml` above the working directory, then `$XDG_CONFIG_HOME/ctl/rgb.yaml` (`~/.config/ctl/rgb.yaml`).
No file means no presets.

```yaml
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
```

`effect` is one of `off`, `static`, `breathing`, `wave`, `rainbow`, and takes the same settings as its command.
An unknown field, a missing colour, or a `default` that names no preset is refused with its line.

## Environment

Every long option reads `RGBCTL_<LONG>` when the flag is absent: `RGBCTL_CONFIG`, `RGBCTL_FORMAT`.
A flag wins over its variable; an empty variable counts as unset.

## Output

Human output by default; `--format json` prints the result as one JSON value with no colour codes.

## Permissions

The node must open read-write: run as root, or install a udev rule that grants a group access to `0db0:0076` hidraw nodes.
An open failure says which.
