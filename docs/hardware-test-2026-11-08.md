# Hardware test, 2026-11-08

Every write and query sent to the MS-7E70's controller on 2026-11-08, in order, with what the operator saw.
Times are BRT (UTC−3), taken from when each command was sent.
Every lighting write was volatile: the save flag stayed `00` throughout.
`docs/protocol.md` holds the facts that came out of it.

## Devices

- JARGB_V2_1: two Cooler Master MF120 Halo fans.
  The controller reports 2 Gen2 strips on this port.
- JARGB_V2_2: the two TL-K12 radiator fans of the Thermalright Core Matrix 240.
- JARGB_V2_3: the Core Matrix 240 pump head.
- JAF_1: nothing connected.

The second run assumed the whole cooler was on JARGB_V2_3; Test G in the fourth run established the map above.
So Tests A to C wrote to the pump head's header, not the radiator fans'.

## First run: rgbctl's commands

| Time | Sent | Read-back | Seen |
|:--|:--|:--|:--|
| 00:04:55 | `rgbctl show` | — | all dark (saved as off) |
| 00:05:24 | `rgbctl static red` | verified | all five devices steady red |
| 00:05:46 | `rgbctl breathing blue` | verified | all blue, fading together |
| 00:06:05 | `rgbctl wave red green blue` | verified | red, green and blue travelling on every device |
| 00:06:49 | `rgbctl rainbow` (mode `05`) | verified | many colours, devices out of sync, each light dark about half the time |
| 00:09:33 | GET_REPORT input `0x01`, read only | — | — |
| 00:09:41 | `rgbctl off` | verified | all dark |

The input `0x01` read returned the first 64 bytes of report `0x50`.

## Second run: Gen2 queries and the cooler

| Time | Sent | Read-back | Seen |
|:--|:--|:--|:--|
| 00:46:20 | Output `0x01`, command `0x82` (Gen2 strip count) for ports 0 to 3 | replies below | not observed separately |
| 00:46:25 | `rgbctl wave` without colours: mode `01`, controller colours, brightness 5, medium | verified | Halo fans: smooth moving rainbow. Cooler: flickering, LEDs white with a slight colour shift, only the top part of each radiator fan lit |
| 00:49:55 | `rgbctl off` | verified | — |
| 00:51:24 | Output `0x01`, command `0x80` (Gen2 mode) for ports 0 to 3 | replies below | — |
| 00:51:32 | Test A: `rgbctl static red`, then the JARGB_V2_3 record's byte 15 raised from `1e` to `b4` (180) | verified | Halo fans red. Cooler white and cyan, only the top light bar of each radiator fan lit |
| 01:10:54 | Test B: report `0x51` to JARGB_V2_3, 80 LEDs in 10-LED colour blocks; its `0x50` record still mode `01` with select-all set | not readable | no change |
| 01:11:36 | Test C: JARGB_V2_3 record set to mode `09`, style `15`, byte 15 `78`; then the same `0x51` blocks | `0x50` verified | no change |
| 01:12:22 | Test D: JARGB_V2_2 and JAF records set the same way; `0x51` all green to JARGB_V2_2 and all blue to JAF | `0x50` verified | not reported |
| 01:13:29 | Report `0x50` restored from the backup taken at the start of Test A (steady red in the all-zones record) | all 290 bytes match | — |
| 01:16:39 | Test E: records 0 to 3 in steady mode `02` with select-all cleared, red, green, blue and yellow, style `35`; all-zones record unchanged | verified | Halo fans cycled colours like a rainbow, not steady red. The cooler was not reported |
| 01:20:21 | Test F setup: `0x51` with length 761, then the `0x50` packet SignalRGB builds for this board (all records mode `09`; records 0 to 3 style `15`, byte 15 `78`) | byte 15 of records 4 to 17 read `00` where `1e` was written; the rest matched | — |
| 01:20:28 to 01:23:28 | Test F loop at 10 Hz: `0x51` per header, red, green, blue and yellow, each followed by GET_REPORT `0x51` | — | no change. Cooler still flashing white, Halo fans on a rainbow |

The device node disappeared about 26 s before the loop ended ("No such device"), when the controller was reset over USB.
After the reset, report `0x50` was restored to its state before any test and `rgbctl static red` was sent: the Halo fans turned red, the cooler stayed white.
The machine was then powered off completely.
After the power cut the controller came up off, its saved state.
`rgbctl static red` was sent with no output `0x01` command before it: the Halo fans and the cooler all turned red.

### Gen2 query replies

Each command was a 64-byte output report: the 7 bytes shown, then zeros.
Each reply was read from the input endpoint; the first 16 bytes are shown.

| Command | Port | Sent | Reply |
|:--|:--|:--|:--|
| `0x82` | 0 | `01 82 00 00 00 00 00` | `01 5a 00 00 00 00 00 02 00 00 00 00 00 00 00 00` |
| `0x82` | 1 | `01 82 00 00 00 00 01` | `01 5a 00 00 00 00 01 00 00 00 00 00 00 00 00 00` |
| `0x82` | 2 | `01 82 00 00 00 00 02` | `01 5a 00 00 00 00 02 00 00 00 00 00 00 00 00 00` |
| `0x82` | 3 | `01 82 00 00 00 00 03` | `01 5a 00 00 00 00 03 00 00 00 00 00 00 00 00 00` |
| `0x80` | 0 | `01 80 00 00 00 00 00` | `01 5a 00 00 00 00 00 01 02 00 00 00 00 00 00 00` |
| `0x80` | 1 | `01 80 00 00 00 00 01` | `01 5a 00 00 00 00 01 00 00 00 00 00 00 00 00 00` |
| `0x80` | 2 | `01 80 00 00 00 00 02` | `01 5a 00 00 00 00 02 00 00 00 00 00 00 00 00 00` |
| `0x80` | 3 | `01 80 00 00 00 00 03` | `01 5a 00 00 00 00 03 00 00 00 00 00 00 00 00 00` |

Byte 7 of a `0x82` reply is the port's Gen2 strip count; byte 7 of a `0x80` reply is 1 when the port is in Gen2 mode.
Byte 8 of port 0's `0x80` reply is `02`, which no source explains.

## Third run: mode sweep after the power cut

The first two rows are rgbctl commands; the first is the post-power-cut `static red` described above.
From 01:33:14 each step wrote the all-zones record (record 17) as a raw report `0x50`, based on a copy of report `0x50` kept before the sweep, with the save flag `00`; every read-back matched.

| Time | Sent to record 17 | Seen |
|:--|:--|:--|
| 01:30:45 | `rgbctl show`, then `rgbctl static red` | every device red, the cooler included |
| 01:32:45 | `rgbctl wave` without colours (mode `01`, style `95`) | smooth on every device, the cooler included |
| 01:33:14 | mode `03` Flame, rainbow colours | only red; the first palette slot was red, so whether Flame uses the palette is not shown |
| 01:34:07 | mode `06` Lightning, user colours red, green, blue, white, style `b5` | far too bright and fast: red, then rapid flashes of the others |
| 01:35:23 | mode `06`, style `a4` (brightness 1, speed slow) | dimmer and slower, so both fields apply |
| 01:36:26 | mode `07` Recreation, brightness 1 | dark or no change |
| 01:36:43 | mode `02`, brightness 1, colour values `30` | dim red, comfortable |
| 01:37:02 | mode `08` Meteor | one colour at a time (red, green, blue, white, repeating), each running along the chain: Halo fans, then radiator fans, then pump head |
| 01:39:46 | mode `0a` GodLike | dark or no change |
| 01:40:18 | mode `05` ColorRing, rainbow colours, style `84` (brightness 1, slow) | not out of sync: the colours follow the chain, Halo fans, radiator fans, then pump head, each device starting just before the previous one finishes |

## Fourth run: direction, per-header records and per-LED

| Time | Sent | Read-back | Seen |
|:--|:--|:--|:--|
| 01:45:11 | Record 17: ColorRing with colours `30 00 00`, `00 30 00`, `00 00 30`, `30 30 30` and style `c4` (as at 01:40:18, with the direction bit, byte 14 bit 6, set) | bytes 273 to 289, record 17 then the save flag: `05 30 00 00 00 30 00 00 00 30 30 30 30 03 c4 00` `00` | the chase ran reversed: pump head, radiator fans, then Halo fans |
| 01:45:40 | Records 0 (JARGB_V2_1) and 2 (JARGB_V2_3): mode `05`, colour count `03`, style `04` (select-all cleared, rainbow colours, brightness 1, slow), byte 15 `1e`; record 17 left as at 01:45:11; records 1 and 3 left at mode `01`, style `94`, select-all set | records 0 and 2 `05 00 00 00 00 00 00 00 00 00 00 00 00 03 04 1e` | "both run, side by side": the Halo fans and the cooler chasing at the same time |
| 01:49:00 | Test G: SignalRGB's MS-7E70 sequence (the `0x50` setup, then the `0x51` loop at 10 Hz for 90 s), colour values `30`: JARGB_V2_1 red, V2_2 green, V2_3 blue, JAF_1 yellow | — | Halo fans red, radiator fans green, pump head blue; nothing yellow |
| 01:58:44 to 02:03:07 | Test H: only JAF_1 (`0x51` type `08`) at `ffff00`, every ARGB header black, stopped early | — | nothing lit |
| 02:03:11 | Report `0x50` restored from the copy taken before any test (lights off) | all 290 bytes match | — |

At 01:45:40 the radiator fans' header (V2_2) still had select-all set and followed record 17, so the "cooler" the operator saw chasing was the pump head on its own record plus the radiator fans on the all-zones record.
The chain order is the header order, JARGB_V2_1, V2_2, V2_3, not the devices' physical order in the case (Halo fans, pump head, radiator fans), and per-LED writes work from a clean state on the Gen2 header and the Gen1 headers alike; JAF_1 is empty.

## What it showed

1. The cooler took steady red in the first run and was white from the start of the second run until the power cut.
   The first commands of the second run were the `0x82` queries, followed 5 s later by a normal wave write.
   The white state outlasted a USB reset of the controller and a restore of report `0x50`, and cleared only after a full power cut.
   So it was volatile controller or device state, not a saved setting.
2. GET_REPORT `0x51` returns report `0x50` data: the first read returned `50 09 ff 00 …`, later reads the same data at a shifted offset.
   A `0x51` read-back therefore cannot confirm a per-LED write.
3. Per-LED writes (`0x51`) work from a clean state on both the Gen2 and the Gen1 headers (Test G); Tests B to D and F lit nothing because the board was in the white state.
4. Per-header records with the select-all bit cleared work from a clean state: ColorRing on records 0 and 2 (style `04`) ran side by side at 01:45:40. In the white state (Test E) the Halo fans showed a rainbow instead of their own steady record.
5. The controller stores `00` in byte 15 of records 4 to 17, whatever is written there.
6. ColorRing (mode `05`, rgbctl's `rainbow`) is a chase along the chain in header order; the first run's "out of sync, dark half the time" was that chase seen at full brightness and medium speed.
7. In the effect modes the controller runs one effect along every header as a single chain, in header order JARGB_V2_1, V2_2, V2_3; the direction bit (style bit 6) reverses it.
8. From a clean state the generated-colour wave is smooth on every device, so the wave write alone did not cause the white state.
9. Recreation (`07`) and GodLike (`0a`) showed nothing; Flame (`03`), Lightning (`06`) and Meteor (`08`) run.

## Open questions

- Whether Flame uses the palette.
- Whether `0x82` put the cooler in the white state.
  The wave write from a clean state did not, which points at `0x82`, but `0x82` was never sent alone.
  rgbctl sends no output `0x01` command, and none is to be sent by hand without a decision.
- Whether anything short of a power cut clears that state.
- Whether per-LED colours stay on the devices after the sender stops.
