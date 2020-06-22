use std::fmt::Debug;
use std::io::{self, Write};
///! RGB Fusion CLI tool
///
/// The Gigabyte RGB Fusion 2 HID protocol information is documentad at
/// https://gitlab.com/CalcProgrammer1/OpenRGB/-/wikis/Gigabyte-RGB-Fusion-2.0.
use std::str::FromStr;

use bytes::{BufMut, BytesMut};
use clap::{arg_enum, crate_authors, crate_description, crate_name, crate_version, App, Arg};
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
            }
            Err(_) => Err(()),
        }
    }
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

// TODO: Better errors
fn main() {
    let config = cli();

    let api = HidApi::new().expect("unable to access HID");
    let device = api
        .open(VENDOR_ID, PRODUCT_ID)
        .expect("unable to access device");

    device
        .write(&config.as_bytes())
        .expect("unable to write effects package");

    device
        .write(&apply_packet())
        .expect("unable to apply changes");
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

    // TODO: Read from stdin when not available
    let color_str = matches.value_of("color").expect("missing color");
    config.color = match Rgb::from_str(color_str) {
        Ok(color) => color,
        _ => panic!("Invalid color '{}'.", color_str),
    };

    config.effect = match matches.value_of("effect") {
        Some(effect) => Effect::from_str(effect).unwrap(),
        None => enum_stdin::<Effect>("Effect", &Effect::variants()),
    };

    if let Some(time) = matches.value_of("fade-in-time").and_then(|time| u16::from_str(time).ok()) {
        config.fade_in_time = Duration(time);
    }

    if let Some(time) = matches.value_of("fade-out-time").and_then(|time| u16::from_str(time).ok()) {
        config.fade_out_time = Duration(time);
    }

    if let Some(time) = matches.value_of("hold-time").and_then(|time| u16::from_str(time).ok()) {
        config.hold_time = Duration(time);
    }

    // TODO: Read from stdin when not available
    let max_brightness_str = matches
        .value_of("max-brightness")
        .expect("missing max brightness");
    config.max_brightness = match u8::from_str(max_brightness_str) {
        Ok(max_brightness) => Brightness(max_brightness),
        _ => panic!("Invalid brightness '{}'.", max_brightness_str),
    };

    if let Some(brightness) = matches
        .value_of("min-brightness")
        .and_then(|mb| u8::from_str(mb).ok())
    {
        config.min_brightness = Brightness(brightness);
    }

    config.zone = match matches.value_of("zone") {
        Some(zone) => Zone::from_str(zone).unwrap(),
        None => enum_stdin::<Zone>("Zone", &Zone::variants()),
    };

    config
}

/// Read enum from STDIN.
fn enum_stdin<T>(name: &str, variants: &[&str]) -> T
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    loop {
        // Offer all available zones.
        println!("[{}] Please select a number:", name);
        for (i, variant) in variants.iter().enumerate() {
            println!("  [{}] {}", i, variant);
        }
        print!(" > ");
        let _ = io::stdout().flush();

        // Read user input.
        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input);
        input = input.trim().to_string();

        match usize::from_str(&input)
            .ok()
            .and_then(|index| variants.get(index))
        {
            Some(variant) => break T::from_str(variant).unwrap(),
            // Query again if the zone is not valid.
            _ => println!("Variant '{}' does not exist, please try again.\n", input),
        }
    }
}
