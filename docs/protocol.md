# Report 0x50 and its effects

What is known about the controller's lighting report, and where each fact comes from.
A fact marked *observed* was seen on an MS-7E70 in the 2026-11-08 hardware test (`docs/hardware-test-2026-11-08.md` lists every write); every other fact cites its source.

MSI's own Mystic Light code is the primary source.
The boardcontrol project decompiled MSI Center's `LEDKeeper2.exe` (classes `MSI_LED.Class_MB_800` and `MSI_LED.MSI_800sLed`) and recorded the report layouts it builds ([MB800 notes][mb800]).
Those notes come from another board of the same family (MS-7E75); the report sizes they list are the ones this board's report descriptor declares.

## Report layout

Feature report `0x50` is 290 bytes: the report id, 18 records of 16 bytes, and the save flag at byte 289.
MSI's code builds it in `Gen1_ApplyBoard` with 18 areas and a store byte at 289 ([MB800 notes, Gen1 apply report][mb800-gen1]).

SignalRGB and OpenRGB name the records in the same order:
JARGB 1 to 3, JAF, JPIPE 1 to 5, JRGB 1 and 2, onboard 1 to 6, and a last record for all zones ("SelectAll").
Sources: SignalRGB's `zoneDict` ([plugin, line 898][srgb-zones]), and the comments of OpenRGB's `initial_setup_array` ([`MSIMotherboard761Controller.cpp`, line 178][orgb-init]).
MSI's `Enum_LedArea` puts JRGB1 at area 9, which agrees ([MB800 notes, zone mapping][mb800-zones]).

rgbctl writes only the last record, bytes 273 to 288, and leaves its byte 288 as read.
*Observed:* writing it changed every connected device.

## Record fields

MSI's `Gen1_ApplyBoard` fills each record as below ([MB800 notes, lines 99 to 110][mb800-fields]).
SignalRGB's `readDeviceZones` decodes the same bytes ([plugin, line 973][srgb-decode]).

| Byte | MSI's field | SignalRGB's reading | rgbctl's name |
|:--|:--|:--|:--|
| 0 | lighting mode, `Enum_LightingMode` | mode | `mode` |
| 1 to 12 | four RGB colours | four RGB colours | `palette` |
| 13 | Option1: colour count, 0 to 3 for 1 to 4 colours | colour count | `sub_parameter` |
| 14 bit 7 | Option2: select-all | "merged with onboard" | `animate` |
| 14 bit 6 | Option2: direction | 0 outward, 1 inward | `direction` (0 right, 1 left) |
| 14 bit 5 | Option2: colour selection | 0 rainbow, 1 user colours | `source` (generated, palette) |
| 14 bits 2 to 4 | Option2: brightness | value × 20 % | `brightness` 1 to 5 |
| 14 bits 0 and 1 | Option2: speed | low, medium, high | `speed` |
| 15 | cycle number | LED count | not owned |

rgbctl's names for bits 6 and 7 come from the reference script, which guessed them; MSI's name for bit 7 is select-all.
For the JARGB V2 headers and the JAF header, MSI Center's `GetCycleNumber` takes the cycle number from the LED count of the Gen2 devices it detected, or from a stored fallback ([zone call-path notes, line 64][mb800-cycle]).

*Observed:* the board holds `1e` (30) as the cycle number of the JARGB and JAF records, and `00` in the last record.
The JARGB, JAF and JRGB1 records hold mode `01` with rainbow colours and style `94`, so their select-all bit is set; the other records are zero.
SignalRGB, in its own per-LED mode, writes `78` (120) as the JARGB cycle number and clears their select-all bit ([plugin, line 940][srgb-template]).
*Observed:* the controller stores `00` in byte 15 of records 4 to 17 whatever is written there, and with its select-all bit cleared the Gen2 header's own steady record did not take effect.

## Modes

MSI's `Enum_LightingMode` ([MB800 notes, line 101][mb800-fields]) and MSI's effect list for this board ([product page][msi-pro]) agree with SignalRGB's `modeDict` ([plugin, line 884][srgb-modes]):

| Byte | MSI | rgbctl command | Observed |
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

MSI's product page lists the effects in the same order, with "CPU Temperature" after Breathing and "Default" last.
CPU Temperature is MSI Center repeatedly writing a steady colour picked from the CPU temperature ([reference script README][ref-readme]), so it has no mode byte.
A live capture of MSI Center on the MS-7E75 recorded steady as `02`, breathing as `04` and off as `00` ([MB800 notes, lines 155 to 162][mb800-live]).
No capture records the other mode bytes.
The older 185-byte boards (report `0x52`) number their modes differently ([OpenRGB `MSI_MODE`][orgb-modes]: 1 static, 2 breathing, 15 color ring, 25 rainbow), and that table does not apply here.
OpenRGB's 761-byte controller for this report family supports only its Direct mode; it notes that "actual support of non direct modes needs to be further investigated" ([line 646][orgb-761-modes]).

## Other reports

The report descriptor declares feature reports `0x51` (727 bytes), `0x90` to `0x93` (302), `0xb0` to `0xb3` (761) and `0xf0` (64), and input and output reports `0x01` and `0xfa` (64 each).
MSI's code uses them as follows ([MB800 notes, report tables][mb800-gen1]):

| Report | MSI's use |
|:--|:--|
| `0x51`, 727 | Gen1 per-LED colours for one area |
| `0x90` + port, 302 | Gen2 apply for one JARGB V2 or JAF port: up to six 20-byte strip records, each with its own mode, colours, options and LED count (180 at most) |
| `0xb0` + port, 761 | Gen2 per-LED colours for one port |
| output `0x01` | commands: firmware version (`0xb0`), Gen2 strip count of a port (`0x82`), Gen2 enable for a port (`0x84`), global switch (`0xba` read, `0xbb` write) |

So MSI's software has a separate Gen2 path, per port, that carries a mode and an LED count for each strip.
A live capture of MSI Center setting JARGB_V2_1 on the MS-7E75 showed only `0x50` traffic; it does not say what was connected to that header ([MB800 notes, lines 18 to 24][mb800-ui]).
SignalRGB sends per-LED colours in `0x51` ([plugin, line 1138][srgb-colors]).
*Observed:* GET_REPORT `0x51` returns report `0x50` data, so a `0x51` read-back cannot confirm a per-LED write, and no `0x51` write lit anything in the 2026-11-08 test, including SignalRGB's full sequence for this board.
*Observed:* the cooler on this board turned white after the `0x82` queries, or the wave write 5 s later, and stayed white through a USB reset and a restore of report `0x50`.
A full power cut is the only known reset: after it, steady red reached the cooler again ([test record][test-record]).
rgbctl never sends an output `0x01` command.
*Observed:* a read-only GET_REPORT for input `0x01` returned the first 64 bytes of report `0x50`, so a read tells nothing about `0x01`.
rgbctl writes none of these reports.

## The mode 05 observation

*Observed:* `rainbow` writes mode `05`, colour count `03`, style `95` (select-all, rainbow colours, brightness 5, speed medium).
It showed many colours, but the devices were out of sync with each other, and each light was dark about as long as it was lit.

Mode `05` is ColorRing in MSI's own enum, so `rgbctl rainbow` sends MSI's ColorRing effect.
No source describes what ColorRing looks like, and no capture records MSI Center sending it.
Three documented facts bear on the observation:

1. The effect runs per header over the record's cycle number, which the board holds as 30 for every JARGB header, while MSI Center sets it from each header's detected LED count.
2. The connected devices differ in length, and probably in generation: the Halo fans are Gen2 (below), and the cooler is likely plain ARGB.
   That is an inference from Thermalright not mentioning Gen2; the other ports reported no Gen2 strip, but they were not in Gen2 mode, so that does not rule a Gen2 device out.
3. MSI documents that a Gen1 device on a JARGB V2 header gets only Gen1 effects ([MSI FAQ 8624][faq-gen1]), that Gen1 and Gen2 devices should not be mixed on one header ([MSI FAQ 8626][faq-mix]), and that Gen2 devices support only 7 of the effects ([product page][msi-pro], which does not name them).

An effect laid over 30 LEDs on chains of other lengths, each starting at its own first LED, would look out of step between devices.
That is an inference from these facts, not a finding.

No source names a mode that gives a steady colour cycle in step across devices.
The per-zone records the board holds by default are mode `01` with rainbow colours at slow speed (`01 … 03 94 1e`).
`rgbctl wave` without colours writes the same mode and colour source to the last record at its default medium speed (style `95`).
*Observed:* that write gave the Halo fans a smooth moving rainbow; the cooler was then in the white state described in the test record, so its look under this effect is not established.

## Connected devices

The test board has two ARGB devices connected:

- JARGB_V2_1: two Cooler Master MF120 Halo fans, chained.
  Each has 24 LEDs, and units made after 1 June 2021 are Addressable Gen 2 RGB ([Cooler Master specification][cm-halo]).
  OpenRGB's sizing table lists the Halo as 24 LEDs wired in parallel ([OpenRGB resize guide][orgb-resize]); whether the chained second fan repeats the first's data or continues it is not established.
- A Thermalright Core Matrix 240, its pump head and two TL-K12 radiator fans on one data line.
  Which header it is on is not established: the 2026-11-08 tests that targeted the JARGB_V2_3 record assumed it ([test record][test-record]).
  Thermalright lists 5 V 3-pin ARGB connectors and does not mention Gen2 ([Core Matrix 240][tr-core], [TL-K12][tr-k12]); no source gives the LED count of the pump or the fans.

The MS-7E70 manual says each JARGB_V2 header takes ARGB Gen2 and ARGB strips, up to 180 LEDs at 3 A, and warns against mixing Gen1 and Gen2 strips on one header ([manual, JARGB_V2 section][msi-manual]).
Gen2 LEDs can report their count back to the controller; plain WS2812-style LEDs cannot ([Gen2 protocol analysis][gen2]).
*Observed:* the controller reports 2 Gen2 strips on JARGB_V2_1, the Halo fans, and none on the other headers, so the Halo fans are Gen2 units ([test record][test-record]).

[test-record]: hardware-test-2026-11-08.md
[mb800]: https://github.com/Blamp26/boardcontrol/blob/5214ed92966bedae5a56383f3a50ea58f07150ce/docs/MSI_7E75_HID_MB800_STATIC_RE.md
[mb800-gen1]: https://github.com/Blamp26/boardcontrol/blob/5214ed92966bedae5a56383f3a50ea58f07150ce/docs/MSI_7E75_HID_MB800_STATIC_RE.md#report--buffer-layout-table
[mb800-fields]: https://github.com/Blamp26/boardcontrol/blob/5214ed92966bedae5a56383f3a50ea58f07150ce/docs/MSI_7E75_HID_MB800_STATIC_RE.md?plain=1#L99-L110
[mb800-live]: https://github.com/Blamp26/boardcontrol/blob/5214ed92966bedae5a56383f3a50ea58f07150ce/docs/MSI_7E75_HID_MB800_STATIC_RE.md?plain=1#L155-L162
[mb800-ui]: https://github.com/Blamp26/boardcontrol/blob/5214ed92966bedae5a56383f3a50ea58f07150ce/docs/MSI_7E75_HID_MB800_STATIC_RE.md?plain=1#L18-L24
[mb800-zones]: https://github.com/Blamp26/boardcontrol/blob/5214ed92966bedae5a56383f3a50ea58f07150ce/docs/MSI_7E75_HID_MB800_STATIC_RE.md#zone-to-report-mapping
[mb800-cycle]: https://github.com/Blamp26/boardcontrol/blob/5214ed92966bedae5a56383f3a50ea58f07150ce/docs/MSI_7E75_ZONE_CALLPATH_STATIC_RE.md?plain=1#L64
[msi-pro]: https://www.msi.com/Motherboard/PRO-X870E-P-WIFI
[msi-manual]: https://download-2.msi.com/archive/mnu_exe/mb/PROX870E-PWIFI_English.pdf
[faq-gen1]: https://www.msi.com/faq/faq-8624
[faq-mix]: https://us.msi.com/faq/faq-8626
[srgb-zones]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L898
[srgb-decode]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L973
[srgb-template]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L940
[srgb-modes]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L884
[srgb-colors]: https://gitlab.com/signalrgb/signal-plugins/-/blob/9511979fa21a0a00a35a236d0aac8c438179094f/Plugins/Msi/185%20Byte%20Motherboard%20Controller/MSI_185Byte_Controller_0x0DB0.js#L1138
[orgb-init]: https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/029758e2260208c6de06eefc7106cd5c462c2310/Controllers/MSIMotherboardController/MSIMotherboard761Controller/MSIMotherboard761Controller.cpp#L178
[orgb-761-modes]: https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/029758e2260208c6de06eefc7106cd5c462c2310/Controllers/MSIMotherboardController/MSIMotherboard761Controller/MSIMotherboard761Controller.cpp#L646
[orgb-modes]: https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/029758e2260208c6de06eefc7106cd5c462c2310/Controllers/MSIMotherboardController/MSIMotherboardCommon.h#L48
[orgb-resize]: https://openrgb.org/resize.html
[ref-readme]: https://github.com/Picachuchu69/msi-mystic-light-b860-x870-linux/blob/166a622ef14f730ed457353f38670ced15a055a3/README.md
[cm-halo]: https://legacy.coolermaster.com/en-global/products/masterfan-mf120-halo/?tab=tech_spec
[tr-core]: https://www.thermalright.com/product/core-matrix-240-white/
[tr-k12]: https://www.thermalright.com/product/tl-k12/
[gen2]: https://github.com/cpldcpu/Gen2-Addressable-RGB/blob/main/docs/Gen2_ARGB_protocol_analysis.md
