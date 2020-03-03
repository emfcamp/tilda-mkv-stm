/// WebUSB module
///
/// This is a modified clone of the usbd-serial crate https://crates.io/crates/usbd-serial

mod buffer;
mod class;
mod device;
mod builder;

pub use usb_device::{Result, UsbError};
pub use crate::webusb::device::*;
