# Report 0x50 and its effects

What is known about the controller's lighting report, and where each fact comes from.
A fact marked *observed* was seen on an MS-7E70 in the 2026-11-08 hardware test; every other fact cites its source.

## Report layout

Feature report `0x50` is 290 bytes: the report id, 18 records of 16 bytes, and the save flag at byte 289.
The report descriptor declares its length; the record split matches the controller's own read-back (`tests/fixtures/report-0x50-off.txt`).

Two independent implementations name the records in the same order:
JARGB 1 to 3, JAF, JPIPE 1 to 5, JRGB 1 and 2, onboard 1 to 6, and a last record for all zones ("SelectAll").
Sources: SignalRGB's `zoneDict` ([plugin, line 898][srgb-zones]) and the comments of OpenRGB's `initial_setup_array` ([`MSIMotherboard761Controller.cpp`, line 178][orgb-init]).

rgbctl writes only the last record, bytes 273 to 288, and leaves its byte 288 as read.
*Observed:* writing it changed every connected device.

## Other reports

The report descriptor declares feature reports `0x51` (727 bytes), `0x90` to `0x93` (302), `0xb0` to `0xb3` (761) and `0xf0` (64), and input and output reports `0x01` and `0xfa` (64 each).
SignalRGB sends per-LED colours in `0x51` ([plugin, line 1138][srgb-colors]).
*Observed:* a read-only GET_REPORT for input `0x01` returned the first 64 bytes of report `0x50`, so a read tells nothing about `0x01`.
rgbctl writes none of these reports.

## Record fields

SignalRGB's `readDeviceZones` decodes each record ([plugin, line 973][srgb-decode]):

| Byte | Field | SignalRGB's reading | rgbctl's name |
|:--|:--|:--|:--|
| 0 | mode | index into the mode table below | `mode` |
| 1 to 12 | four colours | RGB triples | `palette` |
| 13 | sub-parameter | bits 0 and 1: colour count | `sub_parameter` |
| 14 bit 7 | flag | "merged with onboard" | `animate` |
| 14 bit 6 | direction | 0 outward, 1 inward | `direction` (0 right, 1 left) |
| 14 bit 5 | colour source | 0 rainbow, 1 user colours | `source` (generated, palette) |
| 14 bits 2 to 4 | brightness | value × 20 % | `brightness` 1 to 5 |
| 14 bits 0 and 1 | speed | low, medium, high | `speed` |
| 15 | LED count | the zone's LED count | not owned |

rgbctl's names for bits 6 and 7 come from the reference script, which never decoded them from a capture.
*Observed:* the board holds `1e` (30) as the LED count of the JARGB and JAF records, and `00` in the last record.

## Modes

SignalRGB's `modeDict` for this report ([plugin, line 884][srgb-modes]):

| Byte | SignalRGB | rgbctl command | Observed |
|:--|:--|:--|:--|
| `00` | Off | `off` | dark |
| `01` | Wave | `wave` | colours travel along every device |
| `02` | Steady | `static` | one steady colour |
| `03` | Flame | — | — |
| `04` | Breathing | `breathing` | one colour fading in and out, in step |
| `05` | ColorRing | `rainbow` | see below |
| `06` | Lightning | — | — |
| `07` | Recreation | — | — |
| `08` | Meteor | — | — |
| `09` | Advanced | — | — |
| `0a` | GodLike | — | — |

The observed rows match SignalRGB's names for bytes `00`, `01`, `02` and `04`.
SignalRGB itself drives this controller only in Advanced mode (`09`), streaming per-LED colours in report `0x51`.
It never writes the other modes, so their names are its author's labels, not a capture.
The older 185-byte boards (report `0x52`) number their modes differently ([OpenRGB `MSI_MODE`][orgb-modes]: 1 static, 2 breathing, 5 lightning, 25 rainbow), and that table does not apply here.
OpenRGB's 761-byte controller for this report family supports only its Direct mode; it notes that "actual support of non direct modes needs to be further investigated" ([line 646][orgb-761-modes]).
MSI's Mystic Light SDK returns style names at run time and documents no byte values ([SDK PDF, v1.0.0.08][msi-sdk]).

## The mode 05 observation

*Observed:* `rainbow` writes mode `05`, sub-parameter `03`, style `95` (rainbow colours, brightness 5, speed medium).
It showed many colours, but the devices were out of sync with each other, and each light was dark about as long as it was lit.

SignalRGB names mode `05` ColorRing, not rainbow.
The reference script calls it "rainbow wave" ([`rgb-placa.py`][ref-script]) and records no description of how it looks.
No source describes ColorRing's look on this controller, so whether the dark half and the per-device offset belong to the effect is not established.
A ring effect that lights part of each ring and runs on each device from its own first LED would look like this; that is an inference, not a finding.

No source names a mode that gives a steady colour cycle in step across devices.
The per-zone records the board holds by default are mode `01` with rainbow colours (`01 … 03 94 1e`), which is what `rgbctl wave` without colours writes; it is not yet checked on the board.

## Connected devices

The test board drives two headers:

- JARGB_V2_1: two Cooler Master MF120 Halo fans, chained.
  Each has 24 addressable LEDs in two loops ([TechPowerUp review][tpu-halo]); Cooler Master calls them dual-loop ARGB Gen 2.
  OpenRGB's sizing table lists the Halo as 24 LEDs wired in parallel ([OpenRGB resize guide][orgb-resize]); whether the chained second fan repeats the first's data or continues it is not established.
- JARGB_V2_3: a Thermalright Core Matrix 240, its pump head and two radiator fans on one cable.
  Thermalright lists only 5 V 3-pin ARGB connectors ([product page][tr-core]); no source gives its LED count or how the pump and fans are chained.

JARGB_V2 is MSI's ARGB Gen2 header: up to 240 LEDs, and it takes plain ARGB strips too ([MSI manual][msi-manual]).
Gen2 LEDs can report their count back to the controller; plain WS2812-style LEDs cannot ([Gen2 protocol analysis][gen2]).
No source says how the controller lays an effect across a strip whose length differs from the LED count in byte 15.

[srgb-zones]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L898
[srgb-decode]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L973
[srgb-modes]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L884
[srgb-colors]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L1138
[orgb-init]: https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/029758e2260208c6de06eefc7106cd5c462c2310/Controllers/MSIMotherboardController/MSIMotherboard761Controller/MSIMotherboard761Controller.cpp#L178
[orgb-761-modes]: https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/029758e2260208c6de06eefc7106cd5c462c2310/Controllers/MSIMotherboardController/MSIMotherboard761Controller/MSIMotherboard761Controller.cpp#L646
[orgb-modes]: https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/029758e2260208c6de06eefc7106cd5c462c2310/Controllers/MSIMotherboardController/MSIMotherboardCommon.h#L48
[orgb-resize]: https://openrgb.org/resize.html
[msi-sdk]: https://storage-asset.msi.com/files/pdf/Mystic_Light_Software_Development_Kit.pdf
[ref-script]: https://github.com/Picachuchu69/msi-mystic-light-b860-x870-linux/blob/166a622ef14f730ed457353f38670ced15a055a3/rgb-placa.py
[tpu-halo]: https://www.techpowerup.com/review/cooler-master-masterfan-mf120-halo/3.html
[tr-core]: https://www.thermalright.com/product/core-matrix-240-white/
[msi-manual]: https://www.manualslib.com/manual/3288106/Msi-Mag-Series.html?page=45
[gen2]: https://github.com/cpldcpu/Gen2-Addressable-RGB/blob/main/docs/Gen2_ARGB_protocol_analysis.md
