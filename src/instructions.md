# rgbctl agent instructions

`rgbctl` reads and changes the lighting of an MSI Mystic Light controller (USB `0db0:0076`) on Linux.
It finds the controller's `/dev/hidrawN` node by vendor and product id; the node number is never fixed.

## Commands

| Command | Aliases | Purpose |
|:--|:--|:--|
| `show` | — | Print the controller's current lighting, decoded |
| `off` | — | Turn the LEDs off |
| `static` | — | One steady colour |
| `breathing` | — | One colour fading in and out |
| `wave` | — | Colours travelling along the strip; no colours means a generated rainbow |
| `rainbow` | — | The controller's own rainbow |
| `preset` | — | Apply a preset from the config file, or its default preset |
| `presets` | — | List the presets in the config file |
| `instructions` | — | Print the operator contract for this version |

Settings:

- `static` and `breathing` take one colour; `wave` takes up to four, repeated in order to fill the palette, and none means a generated rainbow.
- `-b/--brightness` is 1 to 5; `-s/--speed` is `slow`, `medium` or `fast`; `-d/--direction` is `right` or `left`.

A colour is `#rrggbb`, `rrggbb`, or one of red, green, blue, white, purple, cyan, yellow, orange, pink.
Defaults: brightness 5, speed medium, direction right.

## Writes

- Every write is a preview the controller forgets at power-off.
  `--save` commits it to the controller's flash, which then survives power cycles.
- Do not pass `--save` unless the operator asked for that setting to persist.
- Every write reads report `0x50`, changes only bytes 273 to 287 and the save flag at 289, writes it, and reads it back.
  A read-back that differs is an error naming each differing byte with the value written and the value read.

## Rainbow

On the MS-7E70, `rainbow` shows many colours, with the devices out of sync and each light dark about as long as it is lit.
Tell the operator this when they ask for a rainbow; `wave` with colours is the checked effect for moving colour.

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
