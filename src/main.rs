//! RGB Fusion CLI tool
//!
//! The Gigabyte RGB Fusion 2 HID protocol information is documentad at
//! https://gitlab.com/CalcProgrammer1/OpenRGB/-/wikis/Gigabyte-RGB-Fusion-2.0.

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::{self, Write};
use std::num::ParseIntError;
use std::str::FromStr;

use clap::builder::EnumValueParser;
use clap::{crate_description, crate_name, crate_version, Arg, ArgMatches, Command, ValueEnum};
use hidapi::HidApi;

use crate::asus_strix_x670e_f::AsusRogStrixX670EF;
use crate::controller::HidController;
use crate::gigabyte_trx40_aorus_master::GigabyteTrx40AorusMaster;

mod asus_strix_x670e_f;
mod controller;
mod gigabyte_trx40_aorus_master;

/// Colors used to test the available zones.
const TESTCOLORS: [Rgb; 6] = [
    Rgb { r: 0xff, g: 0x00, b: 0x00 },
    Rgb { r: 0x00, g: 0xff, b: 0x00 },
    Rgb { r: 0x00, g: 0x00, b: 0xff },
    Rgb { r: 0xff, g: 0x00, b: 0xff },
    Rgb { r: 0xff, g: 0xff, b: 0x00 },
    Rgb { r: 0xff, g: 0xff, b: 0xff },
];

/// RGB zone.
#[derive(ValueEnum, Default, Debug, Copy, Clone)]
enum Zone {
    #[default]
    Io,
    Cpu,
    Audio,
    Chipset,
    Header0,
    Header1,
}

/// Color effect.
#[derive(ValueEnum, Default, PartialEq, Eq, Debug, Copy, Clone)]
enum Effect {
    Off,
    #[default]
    Static,
    Pulse,
    Flash,
    Cycle,
    Rainbow,
    ChaseFade,
    Chase,
}

/// Supported RGB controllers.
#[derive(ValueEnum, Default, PartialEq, Eq, Debug, Copy, Clone)]
enum RgbDevice {
    #[default]
    X670EF,
    Trx40,
}

impl RgbDevice {
    /// Get RGB controller for a device.
    fn controller(&self) -> Box<dyn HidController> {
        match self {
            Self::Trx40 => Box::new(GigabyteTrx40AorusMaster),
            Self::X670EF => Box::new(AsusRogStrixX670EF),
        }
    }
}

/// RGB color.
#[derive(Default, Debug, Copy, Clone)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl FromStr for Rgb {
    type Err = ();

    fn from_str(s: &str) -> Result<Rgb, ()> {
        let chars = if s.starts_with("0x") && s.len() == 8 {
            &s[2..]
        } else {
            return Err(());
        };

        match u32::from_str_radix(chars, 16) {
            Ok(mut color) => {
                let b = (color & 0xff) as u8;
                color >>= 8;
                let g = (color & 0xff) as u8;
                color >>= 8;
                let r = color as u8;
                Ok(Rgb { r, g, b })
            },
            Err(_) => Err(()),
        }
    }
}

impl Display for Rgb {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// LED brightness.
#[derive(Default, PartialEq, Eq, Copy, Clone)]
struct Brightness(u8);

impl Brightness {
    const fn max_value() -> Self {
        Self(u8::max_value())
    }
}

impl FromStr for Brightness {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Brightness(u8::from_str(s)?))
    }
}

impl Display for Brightness {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Duration in milliseconds.
#[derive(PartialEq, Eq, Copy, Clone)]
struct Duration(u16);

impl Default for Duration {
    fn default() -> Self {
        Self(100)
    }
}

impl FromStr for Duration {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Duration(u16::from_str(s)?))
    }
}

impl Display for Duration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// New color config.
struct Config {
    device: RgbDevice,
    zone: Zone,
    effect: Effect,
    max_brightness: Brightness,
    min_brightness: Brightness,
    color: Rgb,
    fade_in_time: Duration,
    fade_out_time: Duration,
    hold_time: Duration,
    interactive: bool,
}

impl Config {
    fn from_cli(matches: &ArgMatches) -> Self {
        let mut config = Config::default();

        // Determine if some parameters need to be read from STDIN.
        config.interactive = !matches.contains_id("zone")
            || !matches.contains_id("color")
            || !matches.contains_id("effect");

        config.device = *required_enum::<RgbDevice>(matches, "device");
        config.zone = *required_enum::<Zone>(matches, "zone");
        config.effect = *required_enum::<Effect>(matches, "effect");

        if config.effect != Effect::Off {
            config.color = required_color(matches);
        }

        config.interactive = !matches.contains_id("zone")
            || !matches.contains_id("effect")
            || (!matches.contains_id("color") && config.effect != Effect::Off);

        replace_from_str(&mut config.max_brightness, matches, "max-brightness");
        replace_from_str(&mut config.min_brightness, matches, "min-brightness");
        replace_from_str(&mut config.fade_in_time, matches, "fade-in-time");
        replace_from_str(&mut config.fade_out_time, matches, "fade-out-time");
        replace_from_str(&mut config.hold_time, matches, "hold-time");

        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_brightness: Brightness::max_value(),
            min_brightness: Default::default(),
            fade_out_time: Default::default(),
            fade_in_time: Default::default(),
            interactive: Default::default(),
            hold_time: Default::default(),
            device: Default::default(),
            effect: Default::default(),
            color: Default::default(),
            zone: Default::default(),
        }
    }
}

impl Display for Config {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Add all required parameters.
        write!(
            f,
            "{} \\\n \
            --device {:?} \\\n \
            --zone {:?} \\\n \
            --effect {:?}",
            crate_name!(),
            self.device,
            self.zone,
            self.effect,
        )?;

        // Omit everything if effect is `Off`.
        if self.effect == Effect::Off {
            return Ok(());
        }

        write!(f, " \\\n  --color {}", self.color)?;

        if self.max_brightness != Brightness::max_value() {
            write!(f, " \\\n  --max-brightness {}", self.max_brightness)?;
        }

        // Omit effect config if the color is configured to be static.
        if self.effect == Effect::Static {
            return Ok(());
        }

        if self.min_brightness != Brightness::default() {
            write!(f, " \\\n  --min-brightness {}", self.min_brightness)?;
        }

        if self.fade_in_time != Duration::default() {
            write!(f, " \\\n  --fade-in-time {}", self.fade_in_time)?;
        }

        if self.fade_out_time != Duration::default() {
            write!(f, " \\\n  --fade-out-time {}", self.fade_out_time)?;
        }

        if self.hold_time != Duration::default() {
            write!(f, " \\\n  --hold-time {}", self.hold_time)?;
        }

        Ok(())
    }
}

fn main() {
    let cli = cli();
    match cli.subcommand_matches("zonetest") {
        Some(_) => zonetest(&cli),
        None => rgbfusion(&cli),
    }
}

/// Mark all zones in a unique color.
fn zonetest(matches: &ArgMatches) {
    println!("Are you sure you want to test the available RGB zones?");
    println!("\x1b[31mThis will reset your RGB Fusion configuration\x1b[0m.");
    print!(" [y/N] > ");
    let _ = io::stdout().flush();

    // Abort unless the user agrees to reset their config.
    if stdin_nextline().to_lowercase() != "y" {
        println!("Bailing out.");
        return;
    }

    let device = required_enum::<RgbDevice>(matches, "device");

    println!("\nTesting available RGB zones...\n");

    for (i, zone) in Zone::value_variants().iter().enumerate() {
        let color = TESTCOLORS[i];

        println!("Color for zone {:?}: {}", zone, color);

        let config = Config { color, device: *device, zone: *zone, ..Default::default() };

        if let Err(err) = write_config(&config) {
            eprintln!("Skipping zone: {err}");
        }
    }
}

/// Update RGB Fusion 2 configuration.
fn rgbfusion(matches: &ArgMatches) {
    let config = Config::from_cli(matches);

    // Print CLI example to skip manual configuration.
    if config.interactive {
        println!("\x1b[32mConfiguration successful.\x1b[0m\n");
        println!("To reapply this config, you can run the following command:\n\n{}\n", config);
    }

    match write_config(&config) {
        Ok(()) => println!("\x1b[32mSuccessfully applied changes.\x1b[0m"),
        Err(err) => eprintln!("\x1b[31mError:\x1b[0m {err:?}"),
    }
}

/// Write a config to the HID bus.
fn write_config(config: &Config) -> Result<(), Box<dyn Error>> {
    let controller = config.device.controller();

    let api = HidApi::new().expect("unable to access HID");
    let device = match api.open(controller.vendor_id(), controller.product_id()) {
        Ok(device) => device,
        Err(err) => {
            return Err(format!("unable to open device: {} (root permissions required)", err).into())
        },
    };

    // Get all byte packets required to apply a configuration.
    let bytes = controller.config_bytes(&config)?;

    for packet in bytes {
        if let Err(err) = device.write(&packet) {
            return Err(format!("unable to write new config: {}", err).into());
        }
    }

    Ok(())
}

/// Get clap CLI parameters.
fn cli() -> ArgMatches {
    Command::new(crate_name!())
        .version(crate_version!())
        .author("Christian Duerr <contact@christianduerr.com>")
        .about(crate_description!())
        .subcommand(Command::new("zonetest").about("Test available RGB zones"))
        .arg(
            Arg::new("device")
                .help("RGB device")
                .long("device")
                .short('d')
                .ignore_case(true)
                .value_parser(EnumValueParser::<RgbDevice>::new()),
        )
        .arg(Arg::new("color").help("LED color in RGB [0xRRGGBB]").long("color").short('c'))
        .arg(
            Arg::new("effect")
                .help("Color transition effect")
                .long("effect")
                .short('e')
                .ignore_case(true)
                .value_parser(EnumValueParser::<Effect>::new()),
        )
        .arg(
            Arg::new("fade-in-time")
                .help("Effect fade in time in milliseconds")
                .long("fade-in-time"),
        )
        .arg(
            Arg::new("fade-out-time")
                .help("Effect fade out time in milliseconds")
                .long("fade-out-time"),
        )
        .arg(Arg::new("hold-time").help("Effect hold time in milliseconds").long("hold-time"))
        .arg(
            Arg::new("max-brightness")
                .help("Maximum brightness [possible values: 0..=255]")
                .long("max-brightness")
                .short('b'),
        )
        .arg(
            Arg::new("min-brightness")
                .help("Minimum brightness used for non-static effects [possible values: 0..=255]")
                .long("min-brightness"),
        )
        .arg(
            Arg::new("zone")
                .help("Position of the LED")
                .long("zone")
                .short('z')
                .ignore_case(true)
                .value_parser(EnumValueParser::<Zone>::new()),
        )
        .get_matches()
}

/// Convert a CLI option from the parameter string.
#[inline]
fn cli_from_str<T>(matches: &ArgMatches, name: &str) -> Option<Result<T, <T as FromStr>::Err>>
where
    T: FromStr,
{
    matches.get_one::<String>(name).map(|value| T::from_str(value))
}

/// Replace config value with the CLI parameter if it is present.
#[inline]
fn replace_from_str<T: FromStr>(option: &mut T, matches: &ArgMatches, name: &str) {
    if let Some(Ok(value)) = cli_from_str(matches, name) {
        *option = value;
    }
}

/// Read the color option from CLI or prompt for STDIN if not present.
fn required_color<T: FromStr>(matches: &ArgMatches) -> T {
    match cli_from_str(matches, "color") {
        Some(Ok(value)) => return value,
        Some(Err(_)) => eprintln!("\x1b[31mInvalid CLI color parameter.\x1b[0m\n"),
        _ => (),
    }

    loop {
        // Query the user for the option.
        print!("Please select a color (format: 0xRRGGBB):\n > ");
        let _ = io::stdout().flush();

        let input = stdin_nextline();

        match T::from_str(&input) {
            Ok(value) => {
                println!("");
                break value;
            },
            Err(_) => eprintln!(
                "\x1b[31mColor '{}' does not match format 0xRRGGBB, please try again.\x1b[0m\n",
                input
            ),
        }
    }
}

/// Read an enum option from CLI or prompt for STDIN if not present.
fn required_enum<'a, T>(matches: &'a ArgMatches, name: &str) -> &'a T
where
    T: ValueEnum + Debug + Copy + Sync + Send + 'static,
{
    if let Some(value) = matches.get_one::<T>(name) {
        return value;
    }

    loop {
        // Offer all available zones.
        println!("[{}] Please select a number:", name);
        let variants = T::value_variants();
        for (i, variant) in variants.iter().enumerate() {
            println!("  [{}] {:?}", i, variant);
        }
        print!(" > ");
        let _ = io::stdout().flush();

        let input = stdin_nextline();

        match usize::from_str(&input).ok().and_then(|index| variants.get(index)) {
            Some(variant) => {
                println!("");
                return variant;
            },
            // Query again if the zone is not valid.
            _ => println!("\x1b[31mVariant '{}' does not exist, please try again.\x1b[0m\n", input),
        }
    }
}

/// Read next line from STDIN.
#[inline]
fn stdin_nextline() -> String {
    let mut input = String::new();

    let _ = io::stdin().read_line(&mut input);
    input = input.trim().to_string();

    input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testcolors_match_zones() {
        assert_eq!(Zone::variants().len(), TESTCOLORS.len());
    }
}
