///! RGB Fusion CLI tool
///
/// The Gigabyte RGB Fusion 2 HID protocol information is documentad at
/// https://gitlab.com/CalcProgrammer1/OpenRGB/-/wikis/Gigabyte-RGB-Fusion-2.0.

use hidapi::HidApi;
use bytes::{BytesMut, Buf, BufMut};

const VENDOR_ID: u16 = 0x048d;
const PRODUCT_ID: u16 = 0x8297;

const fn apply_packet() -> [u8; 23] {
    let mut packet = [0; 23];
    packet[0] = 0xcc;
    packet[1] = 0x28;
    packet[2] = 0xff;
    packet
}

/// Color effect.
#[derive(Copy, Clone)]
enum Effect {
    None = 0,
    Static = 1,
    Pulse = 2,
    Flash = 3,
    Cycle = 4,
}

/// LED brightness.
struct Brightness(u8);

/// RGB zones.
#[derive(Copy, Clone)]
enum Zone {
    IO = 0x2001,
    CPU = 0x2102,
    SID = 0x2308,
    CX = 0x2410,
    LED0 = 0x2520,
    LED1 = 0x2640,
}

impl Brightness {
    /// Convert format from 0..=255 to the protocol's range 0..=90.
    fn as_byte(&self) -> u8 {
        (0x5a * self.0 as u16 / u8::max_value() as u16) as u8
    }
}

// TODO: Rename?
//
/// HID package payload data.
struct EffectPacket {
    zone: Zone,
    effect: Effect,
    max_brightness: Brightness,
    min_brightness: Brightness,
    rgb0: [u8; 3],

    // TODO: What does this do?
    rgb1: [u8; 3],

    // TODO: What unit are these?
    fade_in_time: u16,
    fade_out_time: u16,
    hold_time: u16,
}

impl EffectPacket {
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
        buf.put_u8(self.rgb0[2]);
        buf.put_u8(self.rgb0[1]);
        buf.put_u8(self.rgb0[0]);

        // Padding.
        buf.put_u8(0);

        // Secondary color Data.
        buf.put_u8(self.rgb1[2]);
        buf.put_u8(self.rgb1[1]);
        buf.put_u8(self.rgb1[0]);

        // Padding.
        buf.put_u8(0);

        // Color effect timings.
        buf.put_u16(self.fade_in_time);
        buf.put_u16(self.fade_out_time);
        buf.put_u16(self.hold_time);

        buf
    }
}

impl Default for EffectPacket {
    fn default() -> Self {
        Self {
            zone: Zone::IO,
            effect: Effect::Static,
            max_brightness: Brightness(255),
            min_brightness: Brightness(0),
            rgb0: [0; 3],
            rgb1: [0; 3],

            // TODO: These probably shouldn't be 0 by default.
            fade_in_time: 0,
            fade_out_time: 0,
            hold_time: 0,
        }
    }
}

// TODO: Clap / Better errors
fn main() {
    let api = HidApi::new().expect("unable to access HID");
    let device = api.open(VENDOR_ID, PRODUCT_ID).expect("unable to access device");

    let effect = EffectPacket {
        zone: Zone::IO,
        rgb0: [255, 0, 0],
        ..Default::default()
    };
    device.write(&effect.as_bytes()).expect("unable to write effects package");

    let effect = EffectPacket {
        zone: Zone::LED1,
        rgb0: [255, 0, 0],
        max_brightness: Brightness(128),
        ..Default::default()
    };
    device.write(&effect.as_bytes()).expect("unable to write effects package");

    device.write(&apply_packet()).expect("unable to apply changes");
}
