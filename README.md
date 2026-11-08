# rgbctl

Read and change the ARGB lighting of an MSI Mystic Light motherboard controller on Linux.
No MSI software, no OpenRGB, no Python, and no daemon:
the controller keeps a saved setting itself.

Status: in development; no release yet.

## Supported hardware

The USB HID controller `0db0:0076` "MSI MYSTIC LIGHT",
found on recent MSI boards such as the PRO X870E-P WIFI (MS-7E70).
rgbctl finds it by vendor and product id under `/sys/class/hidraw`,
never by a fixed `/dev/hidrawN` number.

## What it does

- `rgbctl show` reads the controller's current lighting report and prints it decoded.
- `rgbctl off`, `static`, `breathing`, `wave` and `rainbow` change it,
  with colours, brightness (1 to 5), speed and direction.
- Every write is a preview the controller forgets at power-off,
  unless `--save` commits it to the controller's flash.
- Every write reads the current report first, changes only the bytes rgbctl owns,
  writes it back, and reads it again to verify.
- Named presets and a default live in `.ctl/rgb.yaml`.

## Permissions

The binary opens the controller's `/dev/hidrawN` node read-write.
Run it as root, or install a udev rule that gives a group access to that node:

```text
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0db0", ATTRS{idProduct}=="0076", GROUP="rgb", MODE="0660"
```

## Credits

The report layout comes from Picachuchu69's
[msi-mystic-light-b860-x870-linux](https://github.com/Picachuchu69/msi-mystic-light-b860-x870-linux)
([`rgb-placa.py` at 166a622](https://github.com/Picachuchu69/msi-mystic-light-b860-x870-linux/blob/166a622ef14f730ed457353f38670ced15a055a3/rgb-placa.py)),
MIT-licensed, which decoded it from USB captures of MSI Center.
rgbctl is a Rust port of its behaviour, not of its code;
`PORTING.md` maps one to the other.

## License

MIT
