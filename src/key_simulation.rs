use std::io::ErrorKind;

use evdev_rs::{
    DeviceWrapper, InputEvent, TimeVal, UInputDevice, UninitDevice,
    enums::{EV_KEY, EV_SYN, EventCode},
};

static KEYS: &[EV_KEY] = &[
    EV_KEY::KEY_NUMLOCK,
    EV_KEY::KEY_BACKSPACE,
    EV_KEY::KEY_ENTER,
    EV_KEY::KEY_SLASH,
    EV_KEY::KEY_KPASTERISK,
    EV_KEY::KEY_MINUS,
    EV_KEY::KEY_KPPLUS,
    EV_KEY::KEY_DOT,
    EV_KEY::KEY_0,
    EV_KEY::KEY_1,
    EV_KEY::KEY_2,
    EV_KEY::KEY_3,
    EV_KEY::KEY_4,
    EV_KEY::KEY_5,
    EV_KEY::KEY_6,
    EV_KEY::KEY_7,
    EV_KEY::KEY_8,
    EV_KEY::KEY_9,
];

pub struct KeySimulator {
    pub udev: UInputDevice,
}

impl KeySimulator {
    const KEY_DOWN: i32 = 1;
    const KEY_UP: i32 = 0;
    pub fn new() -> std::io::Result<Self> {
        let dev = UninitDevice::new().ok_or(std::io::Error::new(
            ErrorKind::Other,
            "could not create an uninitialized device",
        ))?;
        dev.set_name("NumberPad");
        for key in KEYS {
            dev.enable(EventCode::EV_KEY(*key))
                .expect(&format!("could not enable {:?}", key));
        }

        let udev = UInputDevice::create_from_device(&dev)?;
        Ok(Self { udev })
    }

    fn syn(&self) {
        self.udev
            .write_event(&InputEvent::new(
                &TimeVal::new(0, 0),
                &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
                0,
            ))
            .unwrap();
    }

    fn send_key_event(&self, keys: &[EV_KEY], event: i32) {
        for key in keys {
            self.udev
                .write_event(&InputEvent::new(
                    &TimeVal::new(0, 0),
                    &EventCode::EV_KEY(*key),
                    event,
                ))
                .unwrap();
        }
        self.syn();
    }
    pub fn keys_down(&self, keys: &[EV_KEY]) {
        self.send_key_event(keys, Self::KEY_DOWN);
    }

    pub fn keys_up(&self, keys: &[EV_KEY]) {
        self.send_key_event(keys, Self::KEY_UP);
    }

    pub fn keys_press(&self, keys: &[EV_KEY]) {
        self.keys_down(keys);
        self.keys_up(keys);
    }
}
