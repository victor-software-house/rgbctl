# lighting Specification

## Purpose

How rgbctl reads and changes the MSI Mystic Light controller's lighting through HID feature report `0x50`, without disturbing the bytes it does not own.

## Requirements

### Requirement: The controller is found by vendor and product id

rgbctl SHALL find the controller among `/sys/class/hidraw/hidrawN` nodes whose `device/uevent` holds `HID_ID=0003:00000DB0:00000076`, in node-number order, and SHALL use the first node that answers a read of report `0x50`.
rgbctl SHALL NOT assume a fixed node number.

#### Scenario: Two matching nodes

- **WHEN** `hidraw10` and `hidraw2` both belong to `0db0:0076`
- **THEN** rgbctl tries `/dev/hidraw2` first

### Requirement: A write changes only the bytes rgbctl owns

Every write SHALL read report `0x50` first, change only bytes 273 to 287 and the save flag at byte 289, and send every other byte as the controller returned it.

#### Scenario: Static red over a saved-off report

- **WHEN** the controller holds a report whose bytes 273 to 289 are `00 … 00 b5 00 00` and the operator runs `rgbctl static red`
- **THEN** the report sent differs from the one read only in bytes 273 to 285, and byte 289 is `00`

### Requirement: A write is a preview unless saved

A write SHALL set byte 289 to `00` unless the operator passes `--save`, which SHALL set it to `01`.

#### Scenario: Save

- **WHEN** the operator runs `rgbctl off --save`
- **THEN** byte 289 of the report sent is `01`

### Requirement: Every write is verified

After a write rgbctl SHALL read report `0x50` again and SHALL fail, naming each differing offset, when any byte other than the save flag differs from the report sent.

#### Scenario: The controller ignores a write

- **WHEN** the read-back still holds the report from before a write that changed byte 273
- **THEN** rgbctl exits with an error naming offset 273

### Requirement: Presets come from rgb.yaml

rgbctl SHALL read presets from `--config` or `RGBCTL_CONFIG`, else the nearest `.ctl/rgb.yaml` above the working directory, else `$XDG_CONFIG_HOME/ctl/rgb.yaml`.
It SHALL refuse an unknown field, a wave of more than four colours, and a `default` that names no preset.

#### Scenario: Default preset

- **WHEN** the file sets `default: ember` and defines `ember`, and the operator runs `rgbctl preset`
- **THEN** rgbctl writes the `ember` effect
