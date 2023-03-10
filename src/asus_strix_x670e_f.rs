//! ASUS ROG Strix X670E-F Aura control.

use std::error::Error;

use bytes::{BufMut, Bytes, BytesMut};

use crate::controller::HidController;
use crate::{Config, Effect, Rgb, Zone};

const IO_MASK: u8 = 0x04 | 0x02 | 0x01;
const CPU_MASK: u8 = 0x20;
const GPU_MASK: u8 = 0x40;

pub struct AsusRogStrixX670EF;

impl HidController for AsusRogStrixX670EF {
    fn vendor_id(&self) -> u16 {
        0x0B05
    }

    fn product_id(&self) -> u16 {
        0x19AF
    }

    fn config_bytes(&self, config: &Config) -> Result<Vec<Bytes>, Box<dyn Error>> {
        let effect = effect_bytes(config.effect);
        let zone = zone_bytes(config.zone)?;

        // Set LED effect.
        let effect_bytes = Bytes::copy_from_slice(&[0xec, 0x35, zone, 0x00, 0x00, effect]);

        // Set LED color.
        let color_bytes = color_bytes(config.zone, config.color)?;

        // Commit to persist across reboots.
        let commit_bytes = Bytes::copy_from_slice(&[0xec, 0x3f, 0x55]);

        Ok(vec![effect_bytes, color_bytes, commit_bytes])
    }
}

/// Convert effect type to ASUS Aura format.
fn effect_bytes(effect: Effect) -> u8 {
    match effect {
        Effect::Off => 0,
        Effect::Static => 1,
        Effect::Pulse => 2,
        Effect::Flash => 3,
        Effect::Cycle => 4,
        Effect::Rainbow => 5,
        Effect::ChaseFade => 7,
        Effect::Chase => 9,
    }
}

/// Convert zone to ASUS Aura format.
fn zone_bytes(zone: Zone) -> Result<u8, Box<dyn Error>> {
    match zone {
        Zone::Io => Ok(0x00),
        Zone::Header0 => Ok(0x01),
        zone => Err(format!("unsupported zone: {zone:?}").into()),
    }
}

/// Convert zone to ASUS Aura format mask.
fn zone_mask(zone: Zone) -> Result<u8, Box<dyn Error>> {
    match zone {
        Zone::Io => Ok(IO_MASK),
        Zone::Header0 => Ok(CPU_MASK | GPU_MASK),
        zone => Err(format!("unsupported zone: {zone:?}").into()),
    }
}

/// Convert color to ASUS Aura format.
fn color_bytes(zone: Zone, color: Rgb) -> Result<Bytes, Box<dyn Error>> {
    let mut buf = BytesMut::new();

    // Set mask for selecting target LEDs.
    let mask = zone_mask(zone)?;
    buf.put_slice(&[0xec, 0x36, 0x00, mask, 0x00]);

    // Motherboard colors.
    for _ in 0..3 {
        buf.put_u8(color.r);
        buf.put_u8(color.g);
        buf.put_u8(color.b);
    }

    // Padding.
    buf.put_slice(&[0x00; 6]);

    // CPU color.
    buf.put_u8(color.r);
    buf.put_u8(color.g);
    buf.put_u8(color.b);

    // GPU color.
    buf.put_u8(color.r);
    buf.put_u8(color.g);
    buf.put_u8(color.b);

    Ok(buf.freeze())
}
