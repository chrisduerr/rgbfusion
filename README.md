# RGB Fusion 2 CLI

## Disclaimer

This is not an official tool and there is no association with Gigabyte. It has
only been tested with the `Gigabyte TRX40 Aorus Master` motherboard. **Use at
your own risk.**

## Description

The RGB Fusion 2 CLI tool allows you to easily configure your motherboard LEDs
from the CLI, no need for complicated GUIs.

## Usage

```
USAGE:
    rgbfusion [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --color <color>                      LED color in RGB [0xRRGGBB]
    -e, --effect <effect>                    Color transition effect [possible values: Off, Static, Pulse, Flash, Cycle]
        --fade-in-time <fade-in-time>        Effect fade in time in milliseconds
        --fade-out-time <fade-out-time>      Effect fade out time in milliseconds
        --hold-time <hold-time>              Effect hold time in milliseconds
    -b, --max-brightness <max-brightness>    Maximum brightness [possible values: 0..=255]
        --min-brightness <min-brightness>    Minimum brightness used for non-static effects [possible values: 0..=255]
    -z, --zone <zone>                        Position of the LED [possible values: IO, CPU, SID, CX, LED0, LED1]

SUBCOMMANDS:
    help        Prints this message or the help of the given subcommand(s)
    zonetest    Test available RGB zones
```

## Examples

To set the LED on your IO shield to a static red, this is all you need:

```
rgbfusion -z IO -e static -c 0xff0000
```

To identify the zones on your motherboard, you can run the `zonetest`
subcommand. **This will reset your configuration** to use arbitrary colors for
identification.

```
rgbfusion zonetest
```
