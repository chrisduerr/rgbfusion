//! Gigabyte TRX40 Aorus Master RGB Fusion control.

use std::error::Error;

use bytes::{BufMut, Bytes, BytesMut};

use crate::controller::HidController;
use crate::{Brightness, Config, Duration, Effect, Zone};

pub struct GigabyteTrx40AorusMaster;

impl HidController for GigabyteTrx40AorusMaster {
    fn vendor_id(&self) -> u16 {
        0x048d
    }

    fn product_id(&self) -> u16 {
        0x8297
    }

    fn config_bytes(&self, config: &Config) -> Result<Vec<Bytes>, Box<dyn Error>> {
        let mut buf = BytesMut::new();

        // Report ID.
        buf.put_u8(0xcc);

        // RGB Zone.
        buf.put_u16(zone_bytes(config.zone));

        // Padding.
        buf.put_slice(&[0; 8]);

        // Effect.
        buf.put_u8(effect_bytes(config.effect)?);

        // Max Brightness.
        buf.put_slice(&brightness_bytes(config.max_brightness));

        // Min Brightness.
        buf.put_slice(&brightness_bytes(config.min_brightness));

        // Primary color Data.
        buf.put_u8(config.color.b);
        buf.put_u8(config.color.g);
        buf.put_u8(config.color.r);

        // Padding.
        buf.put_u8(0);

        // Secondary color Data.
        buf.put_slice(&[0; 3]);

        // Padding.
        buf.put_u8(0);

        // Color effect timings.
        buf.put_slice(&duration_bytes(config.fade_in_time));
        buf.put_slice(&duration_bytes(config.fade_out_time));
        buf.put_slice(&duration_bytes(config.hold_time));

        // Padding for minimum packet size.
        buf.put_slice(&[0; 3]);

        // Packet to apply the submitted configuration.
        buf.put_u8(0xcc);
        buf.put_u8(0x28);
        buf.put_u8(0xff);
        buf.put_slice(&[0; 20]);

        Ok(vec![buf.freeze()])
    }
}

/// Convert duration to RGB Fusion format.
fn duration_bytes(duration: Duration) -> Bytes {
    let mut bytes = BytesMut::with_capacity(2);

    // Convert from milliseconds to quarter seconds.
    bytes.put_u16(duration.0 / 250);

    bytes.freeze()
}

/// Convert brightness to RGB Fusion format.
fn brightness_bytes(brightness: Brightness) -> Bytes {
    // Convert format from 0..=255 to the protocol's range 0..=90.
    let byte = (0x5a * brightness.0 as u16 / u8::max_value() as u16) as u8;
    Bytes::copy_from_slice(&[byte])
}

/// Convert effect type to RGB Fusion format.
fn effect_bytes(effect: Effect) -> Result<u8, Box<dyn Error>> {
    match effect {
        Effect::Off => Ok(0),
        Effect::Static => Ok(1),
        Effect::Pulse => Ok(2),
        Effect::Flash => Ok(3),
        Effect::Cycle => Ok(4),
        effect => Err(format!("unsupported effect: {effect:?}").into()),
    }
}

/// Convert zone to RGB Fusion format.
fn zone_bytes(zone: Zone) -> u16 {
    match zone {
        Zone::Io => 0x2001,
        Zone::Cpu => 0x2102,
        Zone::Audio => 0x2308,
        Zone::Chipset => 0x2410,
        Zone::Header0 => 0x2520,
        Zone::Header1 => 0x2640,
    }
}
