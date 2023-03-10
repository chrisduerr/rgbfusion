# RGB CLI

## Disclaimer

Garbage hacky CLI for personal use. No guarantees this won't light your house on
fire.

Supported devices:
 - Gigabyte TRX40 Aorus Master
 - ASUS ROG Strix X670E-F

## Description

The RGB CLI tool allows you to easily configure your motherboard LEDs from the
CLI, no need for complicated GUIs.

## Usage

```
RGB CLI tool

Usage: rgbfusion [OPTIONS] [COMMAND]

Commands:
  zonetest  Test available RGB zones
  help      Print this message or the help of the given subcommand(s)

Options:
  -d, --device <device>
          RGB device [possible values: x670ef, trx40]
  -c, --color <color>
          LED color in RGB [0xRRGGBB]
  -e, --effect <effect>
          Color transition effect [possible values: off, static, pulse, flash, cycle, rainbow, chase-fade, chase]
      --fade-in-time <fade-in-time>
          Effect fade in time in milliseconds
      --fade-out-time <fade-out-time>
          Effect fade out time in milliseconds
      --hold-time <hold-time>
          Effect hold time in milliseconds
  -b, --max-brightness <max-brightness>
          Maximum brightness [possible values: 0..=255]
      --min-brightness <min-brightness>
          Minimum brightness used for non-static effects [possible values: 0..=255]
  -z, --zone <zone>
          Position of the LED [possible values: io, cpu, audio, chipset, header0, header1]
  -h, --help
          Print help
  -V, --version
          Print version
```

## Examples

To set the LED on your IO shield to a static red, this is all you need:

```
rgbfusion -d X670EF -z IO -e static -c 0xff0000
```

To identify the zones on your motherboard, you can run the `zonetest`
subcommand. **This will reset your configuration** to use arbitrary colors for
identification.

```
rgbfusion zonetest
```
