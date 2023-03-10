//! RGB controller abstraction.

use std::error::Error;

use bytes::Bytes;

use crate::Config;

/// HID RGB controller.
pub(crate) trait HidController {
    /// HID vendor ID.
    fn vendor_id(&self) -> u16;

    /// HID product ID.
    fn product_id(&self) -> u16;

    /// Convert RGB config to controller-specific bytes.
    fn config_bytes(&self, config: &Config) -> Result<Vec<Bytes>, Box<dyn Error>>;
}
