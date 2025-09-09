use std::{
    io::ErrorKind,
    os::fd::AsRawFd,
    time::{Duration, Instant},
};

use evdev_rs::{
    Device, GrabMode, InputEvent, ReadFlag,
    enums::{EV_ABS, EV_KEY, EV_MSC, EventCode},
};
use libc::{POLLIN, pollfd};
use thiserror::Error;

use crate::{
    key_simulation::KeySimulator,
    layout::{Layout, default_numpad_layout},
    numpad_light::{MAX_BRIGHTNESS, NumpadLight},
};

// TODO:
// Currently, when the numpad is enabled, double touch does not work, so in order to scroll you need
// to use 1 finger and then the other one. Fix.
// We should probably use the ID given to us by evdev

struct TouchPadId {
    i2c_id: u32,
    ev_id: u32,
}
fn get_touchpad_id() -> std::io::Result<TouchPadId> {
    let devices = std::fs::read_to_string("/proc/bus/input/devices")?;
    let mut i2c_id: u32 = 0;
    let mut ev_id: u32 = 0;
    let mut is_in_touchpad_block = false;
    for line in devices.lines() {
        if is_in_touchpad_block {
            if line.starts_with("S:") {
                i2c_id = line
                    .split("i2c-")
                    .nth(1)
                    .unwrap()
                    .chars()
                    .take_while(|c| c.is_numeric())
                    .collect::<String>()
                    .parse()
                    .unwrap();
            } else if line.starts_with("H:") {
                ev_id = line
                    .split("event")
                    .nth(1)
                    .unwrap()
                    .chars()
                    .take_while(|c| c.is_numeric())
                    .collect::<String>()
                    .parse()
                    .unwrap();
                // H appears after the S, so we're done parsing
                break;
            }
        } else {
            is_in_touchpad_block =
                line.starts_with("N:") && line.contains("ASUF") && line.contains("Touchpad");
        }
    }
    if i2c_id == 0 {
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "could not find touchpad i2c ID!",
        ));
    } else if ev_id == 0 {
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "could not find touchpad ev ID!",
        ));
    }

    Ok(TouchPadId { i2c_id, ev_id })
}

#[derive(Debug)]
struct LastTouch {
    pos_x: usize,
    pos_y: usize,
    time: Instant,
    key: Option<EV_KEY>,
}
#[derive(Debug)]
pub struct NumpadState {
    pos_x: usize,
    pos_y: usize,
    last_touch: LastTouch,
    is_active: bool,
    is_dragging: bool,
    is_lifted: bool,
}

impl NumpadState {
    fn new() -> Self {
        Self {
            pos_x: 0,
            pos_y: 0,
            last_touch: LastTouch {
                pos_x: 0,
                pos_y: 0,
                time: Instant::now(),
                key: None,
            },
            is_active: false,
            is_dragging: false,
            is_lifted: true,
        }
    }
}

pub struct NumberPad {
    touchpad: Device,
    key_simulator: KeySimulator,
    light_controller: NumpadLight,
    state: NumpadState,
    layout: Layout<EV_KEY>,
    holding_key: Option<EV_KEY>,
    brightness: u8,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not find touchpad id; error: {}", .0)]
    TouchpadNotFound(std::io::Error),
    #[error("Couldn't open touchpad device {}, error: {}", .device_name, .error)]
    CouldntOpenTouchpaddDevice {
        device_name: String,
        error: std::io::Error,
    },
    #[error("Couldn't connect to the numpad's light: {}", .0)]
    CouldntConnectToNumpadLight(i2cdev::linux::LinuxI2CError),
    #[error("Couldn't create keyboard device: {}", .0)]
    CouldntCreateKeyboardDevice(std::io::Error),
}

impl NumberPad {
    const MIN_DRAG_DISTANCE: f64 = 30.0;
    const HOLD_DURATION: Duration = Duration::from_millis(250);
    pub fn new() -> std::result::Result<Self, Error> {
        let ids = get_touchpad_id().map_err(Error::TouchpadNotFound)?;
        let device_path = format!("/dev/input/event{}", ids.ev_id);
        let touchpad =
            Device::new_from_path(&device_path).map_err(|e| Error::CouldntOpenTouchpaddDevice {
                device_name: device_path.to_string(),
                error: e,
            })?;
        let mut light_controller =
            NumpadLight::new(ids.i2c_id).map_err(Error::CouldntConnectToNumpadLight)?;
        let key_simulator = KeySimulator::new().map_err(Error::CouldntCreateKeyboardDevice)?;
        light_controller.turn_off().unwrap();
        light_controller.set_brightness(MAX_BRIGHTNESS).unwrap();
        Ok(Self {
            touchpad,
            key_simulator,
            light_controller,
            state: NumpadState::new(),
            layout: default_numpad_layout(),
            holding_key: None,
            brightness: MAX_BRIGHTNESS,
        })
    }

    fn stop_holding_key(&mut self) {
        if let Some(key) = self.holding_key {
            self.key_simulator.keys_up(&[key]);
            self.holding_key = None;
        }
    }

    fn is_drag_down(&self) -> bool {
        if self.state.pos_y > self.state.last_touch.pos_y {
            true
        } else {
            false
        }
    }

    fn is_drag_up(&self) -> bool {
        if self.state.pos_y < self.state.last_touch.pos_y {
            true
        } else {
            false
        }
    }
    fn lift(&mut self) {
        if self.state.is_dragging {
            self.state.is_dragging = false;
            // if the drag started in the numlock area it means we should adjust the brightness
            if self.state.is_active && self.state.last_touch.key == Some(EV_KEY::KEY_NUMLOCK) {
                if self.is_drag_up() {
                    if self.brightness < MAX_BRIGHTNESS {
                        self.brightness += 1;
                        self.light_controller
                            .set_brightness(self.brightness)
                            .unwrap();
                    }
                } else if self.is_drag_down() {
                    if self.brightness > 0 {
                        self.brightness -= 1;
                        self.light_controller
                            .set_brightness(self.brightness)
                            .unwrap();
                    }
                }
                // we didn't stop the grab if it started from
                self.touchpad.grab(evdev_rs::GrabMode::Grab).unwrap();
            }

            return;
        } else if self.holding_key.is_some() {
            self.stop_holding_key();
            return;
        } else if let Some(key) = self.layout.get_item(self.state.pos_x, self.state.pos_y) {
            match key {
                EV_KEY::KEY_NUMLOCK => {
                    self.state.is_active = !self.state.is_active;
                    // numlock integration?
                    //self.key_simulator.keys_press(&[EV_KEY::KEY_NUMLOCK]);
                    if self.state.is_active {
                        self.light_controller.turn_on().unwrap();
                    } else {
                        self.light_controller.turn_off().unwrap();
                        // we might still be grabbing if the user hasn't done a drag; ensure we ungrab
                        self.touchpad.grab(GrabMode::Ungrab).unwrap();
                    }
                }
                _ => {
                    if self.state.is_active {
                        //  press the desired key
                        self.key_simulator.keys_press(&[key])
                    }
                }
            }
        }
    }
    fn handle_touchpad_event(&mut self, event: InputEvent) {
        match event.event_code {
            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_X) => {
                self.state.pos_x = event.value as usize;
            }
            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_Y) => {
                self.state.pos_y = event.value as usize;
            }
            EventCode::EV_KEY(EV_KEY::BTN_TOOL_FINGER) => {
                if event.value == 0 {
                    // finger lifted
                    self.state.is_lifted = true;
                    self.lift();
                } else {
                    if self.state.is_dragging {
                        // if we're dragging, it means the user has a hand on the touchpad
                        // and its likely they're trying to do some kind of gesture, so we don't need to grab anything.
                        return;
                    }
                    // finger is on the touchpad
                    self.state.last_touch.pos_x = self.state.pos_x;
                    self.state.last_touch.pos_y = self.state.pos_y;
                    self.state.last_touch.time = Instant::now();
                    self.state.is_lifted = false;
                    self.state.last_touch.key =
                        self.layout.get_item(self.state.pos_x, self.state.pos_y);
                    if self.state.is_active
                        && // if the user touches a place which is not in the layout it is considered as normal mouse movement; we don't need to grab.
                        self.state.last_touch.key.is_some()
                    {
                        // NOTE: MUST ACTIVATE THE GRAB HERE RATHER THAN SIMPLY GRABBING WHEN ENABLED
                        // AND THEN UNGRABBING/GRABBING WHEN NECESSARY.
                        // IF WE GRAB WHEN ENABLED, DRAGGING WON'T WORK FOR SOME REASON.
                        self.touchpad.grab(evdev_rs::GrabMode::Grab).unwrap();
                    }
                }
            }

            EventCode::EV_MSC(EV_MSC::MSC_TIMESTAMP) => {
                // the user is holding; check if they moved far enough from the first touch
                fn dist(x1: usize, y1: usize, x2: usize, y2: usize) -> f64 {
                    ((x1 as f64 - x2 as f64).powi(2) + (y1 as f64 - y2 as f64).powi(2)).sqrt()
                }
                if self.state.is_lifted {
                    return;
                }
                if !self.state.is_dragging
                    && dist(
                        self.state.pos_x,
                        self.state.pos_y,
                        self.state.last_touch.pos_x,
                        self.state.last_touch.pos_y,
                    ) >= Self::MIN_DRAG_DISTANCE
                {
                    // if the touched key is numlock, it means the user is trying to change the brightness,
                    // so we don't need to release the grab on the touchpad
                    if self.state.last_touch.key != Some(EV_KEY::KEY_NUMLOCK) {
                        // the user wants to move the cursor; ungrab
                        self.touchpad.grab(GrabMode::Ungrab).unwrap();
                    }
                    self.state.is_dragging = true;
                    self.stop_holding_key();
                } else if self.state.is_active
                    && !self.state.is_dragging
                    && Instant::now() - self.state.last_touch.time > Self::HOLD_DURATION
                    && self.holding_key.is_none()
                {
                    if let Some(key) = self.state.last_touch.key {
                        self.holding_key = Some(key);
                        match key {
                            EV_KEY::KEY_NUMLOCK => {
                                // do something
                            }

                            _ => self.key_simulator.keys_down(&[key]),
                        }
                    }
                }
            }
            _ => (),
        }
    }
    pub fn enter_input_loop(&mut self) -> std::io::Result<()> {
        let mut fds = pollfd {
            fd: self.touchpad.file().as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };
        loop {
            // wait for some event to happen so that we don't busywait2
            unsafe {
                let result = libc::poll(&mut fds, 1, -1);
                if result < 0 {
                    panic!("error: {}", std::io::Error::last_os_error());
                }
            }

            // read all the events that happened
            while let Ok((_read_flags, event)) = self.touchpad.next_event(ReadFlag::NORMAL) {
                self.handle_touchpad_event(event);
            }
        }
    }
}
