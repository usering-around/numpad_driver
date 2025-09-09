use std::io::ErrorKind;

use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};

pub struct NumpadLight {
    dev: LinuxI2CDevice,
}

type Result<T> = std::result::Result<T, i2cdev::linux::LinuxI2CError>;

pub const MAX_BRIGHTNESS: u8 = 7;

impl NumpadLight {
    const TURN_OFF: u8 = 0;
    const TURN_ON: u8 = 1;
    const BRIGHTNESS_OFFSET: u8 = 65;
    pub fn new(i2c_id: u32) -> Result<Self> {
        let slave_addr = 0x38;
        // we need to force it bc the driver is constatnly busy. This should be fine since the current driver doesn't even touch the brightness anyways.
        let dev = unsafe { LinuxI2CDevice::force_new(format!("/dev/i2c-{}", i2c_id), slave_addr)? };
        Ok(Self { dev })
    }

    fn write(&mut self, num: u8) -> Result<()> {
        self.dev.write(&[
            0x05, 0x00, 0x3d, 0x03, 0x06, 0x00, 0x07, 0x00, 0x0d, 0x14, 0x03, num, 0xad,
        ])
    }

    /// Turn on the numpad light. If the numpad is not turned on, setting the brightness won't do anything.
    pub fn turn_on(&mut self) -> Result<()> {
        self.write(Self::TURN_ON)
    }

    /// Turn off the numpad light.
    pub fn turn_off(&mut self) -> Result<()> {
        self.write(Self::TURN_OFF)
    }

    /// Set the brightness level, assuming the numpad is turned on.
    /// Will return an error if the given brightness num is greater than the MAX_BRIGHTNESS constant,
    /// or if some IO error occured.
    pub fn set_brightness(&mut self, brightness_num: u8) -> Result<()> {
        if brightness_num > MAX_BRIGHTNESS {
            Err(std::io::Error::new(
                ErrorKind::Other,
                "brightness number exceeded; max is 14",
            ))?;
        }

        self.write(brightness_num + Self::BRIGHTNESS_OFFSET)
    }
}
