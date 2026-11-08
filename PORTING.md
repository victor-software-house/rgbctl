# Porting notes

rgbctl ports the behaviour of `rgb-placa.py` from
[msi-mystic-light-b860-x870-linux](https://github.com/Picachuchu69/msi-mystic-light-b860-x870-linux/blob/166a622ef14f730ed457353f38670ced15a055a3/rgb-placa.py)
(MIT) at commit 166a622.
It ports what the script sends, not how the script is built.

## API mapping

| Original | rgbctl |
|:--|:--|
| `_MODE_IDX`, `MODE_*` | `report::Mode`, byte 273 |
| `_COLOR_IDX`, the four colour slots | `Lighting::palette`, bytes 274 to 285 |
| `_SUB_IDX` | `Lighting::sub_parameter`, byte 286 |
| `_param()`, `_PARAM_IDX` | `report::Style` with `TryFrom<u8>` and `From<Style> for u8`, byte 287 |
| `_SAVE_IDX`, `guardar` | `report::Persistence`, byte 289 |
| `_template()`, `_TEMPLATE_HEX` | none: `hidraw::Controller::read` returns the controller's own report |
| `set_argb()` | `write::plan`, then `Controller::write`, then `write::verify` |
| `find_device()` | `hidraw::find` |
| `HIDIOCSFEATURE` lambda | `hidraw` `SET_FEATURE`, and `GET_FEATURE` for reads |
| `COLORS`, `_color()` | `color::NAMED`, `Rgb::from_str` |
| `SPEEDS`, `DIRS` | `report::Speed`, `report::Direction` |
| `respira`, `ola`, `arcoiris`, `off`, colour and `R G B` commands | `rgbctl breathing`, `wave`, `rainbow`, `off`, `static` |
| `temporal` / `volatil` word | inverted: writes are previews unless `--save` |

## Test mapping

The script has no tests.
The characterization tests come from its behaviour and from a dump of report `0x50` read from a board whose saved setting was off (`tests/fixtures/report-0x50-off.txt`).

| Behaviour | Test |
|:--|:--|
| The off command writes `00` mode, black palette, `00`, `b5` | `effect::tests::off_writes_what_the_saved_off_dump_holds`, `report::tests::the_saved_off_dump_decodes_as_off_at_full_brightness` |
| Byte 287 formula | `report::tests::the_style_formula_matches_the_reference` |
| Static and breathing repeat one colour in every slot | `effect::tests::static_repeats_its_colour_in_every_slot`, `breathing_carries_its_speed` |
| A wave of colours sets bit 5 and sub-parameter `03` | `effect::tests::a_wave_of_colours_reads_the_palette` |
| A wave without colours, and rainbow, clear bit 5 | `effect::tests::a_wave_without_colours_generates_them_and_keeps_the_palette`, `rainbow_generates_colours_and_keeps_the_palette` |
| Brightness 1 to 5, colour values 0 to 255 | `report::tests::brightness_accepts_one_to_five`, `cli::tests::brightness_outside_one_to_five_is_refused`, `color::tests` |
| The device is found by `0db0:0076` | `hidraw::tests::find_returns_the_controller_nodes_in_numeric_order` |

## Deliberate differences

- **No template.** The script sends a 290-byte report captured from MSI Center, overwriting every byte it does not own.
  rgbctl reads the controller's report and writes back every byte it does not own unchanged.
- **Preview by default.** The script saved to flash unless told `temporal`; rgbctl saves only with `--save`.
- **Read-back check.** Every write is read back and compared; the script never read.
- **Two or three wave colours** repeat in order to fill four slots (`red blue` writes red, blue, red, blue).
  The script left the remaining slots as its template held them.
- **A wave without colours and rainbow keep the palette** the controller holds, where the script wrote its template's.
  The palette is unused while bit 5 is clear.
- **Device discovery** matches the `HID_ID` line exactly and orders nodes by number, trying each until one answers report `0x50`.
  The script took the first path in string order whose `uevent` contained both ids anywhere, so `hidraw10` sorted before `hidraw2`.
- **English command and colour names.** `purpura`, a second name for purple, is dropped.
- **Not ported:** `estandar` (a still multicolour wave), `on` (static `#404040`), and `temp`, which sets a colour from the CPU temperature.
  `temp bucle` needs a running loop, and rgbctl runs no daemon; the other two are presets a config file can hold.

## Preserved quirks

- Every effect sets bit 7 (animate), static included, as the script does.
- Static and off write speed medium (`1`) in byte 287, as the script does.
- Off writes brightness 5 and bit 5 set (`b5`), which matches the dump of a board saved as off.

## Verification

Tests run in the pull request's CI, not on a developer machine.
Hardware checks on the board are listed in the pull request that records them.
