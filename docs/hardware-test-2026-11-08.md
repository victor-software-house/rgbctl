# Hardware test, 2026-11-08

Every write and query sent to the MS-7E70's controller on 2026-11-08, in order, with what the operator saw.
Times are BRT (UTC−3), taken from when each command was sent.
Every lighting write was volatile: the save flag stayed `00` throughout.
`docs/protocol.md` holds the facts that came out of it.

## Devices

- JARGB_V2_1: two Cooler Master MF120 Halo fans.
  The controller reports 2 Gen2 strips on this port.
- The Thermalright Core Matrix 240 (pump head and two TL-K12 radiator fans) is on one data line.
  Which header it sits on is not established; see the Open questions section.

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

## What it showed

1. The cooler took steady red in the first run and was white from the start of the second run until the power cut.
   The first commands of the second run were the `0x82` queries, followed 5 s later by a normal wave write.
   The white state outlasted a USB reset of the controller and a restore of report `0x50`, and cleared only after a full power cut.
   So it was volatile controller or device state, not a saved setting.
2. GET_REPORT `0x51` returns report `0x50` data: the first read returned `50 09 ff 00 …`, later reads the same data at a shifted offset.
   A `0x51` read-back therefore cannot confirm a per-LED write.
3. No `0x51` per-LED write lit anything, including the full sequence SignalRGB runs for this board.
4. With its select-all bit cleared, the Gen2 header's own steady record did not take effect; the Halo fans showed a rainbow.
5. The controller stores `00` in byte 15 of records 4 to 17, whatever is written there.

## Open questions

- Which header the cooler is on, and how it is chained.
- Whether `0x82` put the cooler in the white state, or the wave write at 00:46:25 did; nothing recorded tells them apart.
  rgbctl sends no output `0x01` command, and none is to be sent by hand without a decision.
- Whether anything short of a power cut clears that state.
