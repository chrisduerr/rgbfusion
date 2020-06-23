//! RGB Fusion CLI tool
//!
//! The Gigabyte RGB Fusion 2 HID protocol information is documentad at
//! https://gitlab.com/CalcProgrammer1/OpenRGB/-/wikis/Gigabyte-RGB-Fusion-2.0.

use std::fmt::{self, Debug, Display, Formatter};
use std::io::{self, Write};
use std::num::ParseIntError;
use std::process::exit;
use std::str::FromStr;

use bytes::{BufMut, BytesMut};
use clap::{arg_enum, crate_description, crate_name, crate_version, App, Arg, ArgMatches};
use hidapi::HidApi;

const VENDOR_ID: u16 = 0x048d;
const PRODUCT_ID: u16 = 0x8297;

const fn apply_packet() -> [u8; 23] {
    let mut packet = [0; 23];
    packet[0] = 0xcc;
    packet[1] = 0x28;
    packet[2] = 0xff;
    packet
}

macro_rules! die {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        exit(1);
    }}
}

arg_enum! {
    /// Color effect.
    #[derive(Debug, Copy, Clone)]
    enum Effect {
        Off = 0,
        Static = 1,
        Pulse = 2,
        Flash = 3,
        Cycle = 4,
    }
}

arg_enum! {
    /// RGB zones.
    #[derive(Debug, Copy, Clone)]
    enum Zone {
        IO = 0x2001,
        CPU = 0x2102,
        SID = 0x2308,
        CX = 0x2410,
        LED0 = 0x2520,
        LED1 = 0x2640,
    }
}

/// TODO: Doc
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
#[derive(Copy, Clone)]
struct Brightness(u8);

impl Brightness {
    // TODO: Make this a trait?
    //
    /// Convert format from 0..=255 to the protocol's range 0..=90.
    fn as_byte(self) -> u8 {
        (0x5a * self.0 as u16 / u8::max_value() as u16) as u8
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
#[derive(Copy, Clone)]
struct Duration(u16);

impl Duration {
    // TODO: Make this a trait?
    //
    /// Convert from milliseconds to quarter seconds.
    fn as_bytes(self) -> u16 {
        self.0 / 250
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
    zone: Zone,
    effect: Effect,
    max_brightness: Brightness,
    min_brightness: Brightness,
    color: Rgb,
    fade_in_time: Duration,
    fade_out_time: Duration,
    hold_time: Duration,
}

impl Config {
    // TODO: Make this a trait?
    //
    /// Convert config to RGB Fusion 2 HID packet.
    fn as_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::new();

        // Report ID.
        buf.put_u8(0xcc);

        // RGB Zone.
        buf.put_u16(self.zone as u16);

        // Padding.
        buf.put_slice(&[0; 8]);

        // Effect.
        buf.put_u8(self.effect as u8);

        // Max Brightness.
        buf.put_u8(self.max_brightness.as_byte());

        // Min Brightness.
        buf.put_u8(self.min_brightness.as_byte());

        // Primary color Data.
        buf.put_u8(self.color.b);
        buf.put_u8(self.color.g);
        buf.put_u8(self.color.r);

        // Padding.
        buf.put_u8(0);

        // Secondary color Data.
        buf.put_slice(&[0; 3]);

        // Padding.
        buf.put_u8(0);

        // Color effect timings.
        buf.put_u16(self.fade_in_time.as_bytes());
        buf.put_u16(self.fade_out_time.as_bytes());
        buf.put_u16(self.hold_time.as_bytes());

        // Padding for minimum packet size.
        buf.put_slice(&[0; 3]);

        buf
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            zone: Zone::IO,
            effect: Effect::Static,
            max_brightness: Brightness(255),
            min_brightness: Brightness(0),
            color: Rgb::default(),
            fade_in_time: Duration(100),
            fade_out_time: Duration(100),
            hold_time: Duration(100),
        }
    }
}

impl Display for Config {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // TODO: Omit optional parameters matching the defaults
        write!(
            f,
            "  {} \\\n    \
            --zone {} \\\n    \
            --color {} \\\n    \
            --effect {} \\\n    \
            --max-brightness {} \\\n    \
            --min-brightness {} \\\n    \
            --fade-in-time {} \\\n    \
            --fade-out-time {} \\\n    \
            --hold-time {}",
            crate_name!(),
            self.zone,
            self.color,
            self.effect,
            self.max_brightness,
            self.min_brightness,
            self.fade_in_time,
            self.fade_out_time,
            self.hold_time
        )
    }
}

// TODO: Add colortest mode to check zone
//  -> Maybe even allow interactively selecting the zone?
fn main() {
    let config = cli();

    let api = HidApi::new().expect("unable to access HID");
    let device = match api.open(VENDOR_ID, PRODUCT_ID) {
        Ok(device) => device,
        Err(err) => die!("unable to open device: {} (root permissions required)", err),
    };

    if let Err(err) = device.write(&config.as_bytes()) {
        die!("unable to write new config: {}", err);
    }

    if let Err(err) = device.write(&apply_packet()) {
        die!("unable to apply new config: {}", err);
    }

    // TODO: Move before HID access in case of failure
    // TODO: Omit if all parameters were specified as parameters already
    println!("\x1b[32mSuccessfully applied changes.\x1b[0m\n");
    println!("The following command will reapply your config:\n\n{}", config);
}

/// Read config from command line.
fn cli() -> Config {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author("Christian Duerr <contact@christianduerr.com>")
        .about(crate_description!())
        .arg(
            Arg::with_name("color")
                .help("LED color in RGB [0xRRGGBB]")
                .long("color")
                .short("c")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("effect")
                .help("Color transition effect")
                .long("effect")
                .short("e")
                .possible_values(&Effect::variants())
                .takes_value(true)
                .case_insensitive(true),
        )
        .arg(
            Arg::with_name("fade-in-time")
                .help("Effect fade in time in milliseconds")
                .long("fade-in-time")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("fade-out-time")
                .help("Effect fade out time in milliseconds")
                .long("fade-out-time")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("hold-time")
                .help("Effect hold time in milliseconds")
                .long("hold-time")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-brightness")
                .help("Maximum brightness [possible values: 0..=255]")
                .long("max-brightness")
                .short("b")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("min-brightness")
                .help("Minimum brightness used for non-static effects [possible values: 0..=255]")
                .long("min-brightness")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("zone")
                .help("Position of the LED")
                .long("zone")
                .short("z")
                .possible_values(&Zone::variants())
                .takes_value(true)
                .case_insensitive(true),
        )
        .get_matches();

    let mut config = Config::default();

    config.zone = required_enum(&matches, "zone", &Zone::variants());

    config.color = required_from_str(&matches, "color", "0xRRGGBB");

    config.effect = required_enum(&matches, "effect", &Effect::variants());

    config.max_brightness = required_from_str(&matches, "max-brightness", "[0..=255]");

    replace_from_str(&mut config.min_brightness, &matches, "min-brightness");
    replace_from_str(&mut config.fade_in_time, &matches, "fade-in-time");
    replace_from_str(&mut config.fade_out_time, &matches, "fade-out-time");
    replace_from_str(&mut config.hold_time, &matches, "hold-time");

    config
}

/// Convert a CLI option from the parameter string.
#[inline]
fn cli_from_str<T>(matches: &ArgMatches, name: &str) -> Option<Result<T, <T as FromStr>::Err>>
where
    T: FromStr,
{
    matches.value_of(name).map(|value| T::from_str(value))
}

/// Replace config value with the CLI parameter if it is present.
#[inline]
fn replace_from_str<T: FromStr>(option: &mut T, matches: &ArgMatches, name: &str) {
    if let Some(Ok(value)) = cli_from_str(matches, name) {
        *option = value;
    }
}

/// Get a required config option from CLI or STDIN.
fn required_from_str<T: FromStr>(matches: &ArgMatches, name: &str, format: &str) -> T {
    match cli_from_str(matches, name) {
        Some(Ok(value)) => return value,
        Some(Err(_)) => eprintln!("\x1b[31mInvalid CLI {} parameter.\x1b[0m\n", name),
        _ => (),
    }

    loop {
        // Query the user for the option.
        print!("Please select a {} (format: {}):\n > ", name, format);
        let _ = io::stdout().flush();

        let input = stdin_nextline();

        match T::from_str(&input) {
            Ok(value) => {
                println!("");
                break value;
            },
            Err(_) => eprintln!(
                "\x1b[31mValue '{}' does not match format {}, please try again.\x1b[0m\n",
                input, format
            ),
        }
    }
}

/// Get a required config enum option.
fn required_enum<T>(matches: &ArgMatches, name: &str, variants: &[&str]) -> T
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    if let Some(Ok(value)) = cli_from_str(matches, name) {
        return value;
    }

    loop {
        // Offer all available zones.
        println!("[{}] Please select a number:", name);
        for (i, variant) in variants.iter().enumerate() {
            println!("  [{}] {}", i, variant);
        }
        print!(" > ");
        let _ = io::stdout().flush();

        let input = stdin_nextline();

        match usize::from_str(&input).ok().and_then(|index| variants.get(index)) {
            Some(variant) => {
                println!("");
                break T::from_str(variant).unwrap();
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
