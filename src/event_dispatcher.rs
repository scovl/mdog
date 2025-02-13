use crate::types::*;

use interception as ic;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::sync::mpsc;

#[derive(Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    Toggle_Key: ic::ScanCode,
    Excluded_Keys: Vec<ic::ScanCode>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            Toggle_Key: ic::ScanCode::Grave,
            Excluded_Keys: vec![
                ic::ScanCode::X,
                ic::ScanCode::J,
                ic::ScanCode::L,
                ic::ScanCode::Z,
                ic::ScanCode::LeftAlt,
                ic::ScanCode::LeftShift,
                ic::ScanCode::Tab,
            ],
        }
    }
}

pub struct EventDispatcher {
    Settings: Settings,
    tx: mpsc::Sender<Event>,
    interception: ic::Interception,
    active: bool,
    key_states: HashMap<(ic::Device, ic::ScanCode), KeyState>,
    mouse_button_states: HashMap<(ic::Device, MouseButton), KeyState>,
}

impl EventDispatcher {
    pub fn new(tx: mpsc::Sender<Event>, Settings: Settings) -> Option<Self> {
        let interception = match ic::Interception::new() {
            Some(interception) => interception,
            None => {
                error!("could not create interception context");
                return None;
            }
        };

        interception.set_filter(
            ic::is_mouse,
            ic::Filter::MouseFilter(ic::MouseFilter::all()),
        );

        interception.set_filter(
            ic::is_keyboard,
            ic::Filter::KeyFilter(ic::KeyFilter::UP | ic::KeyFilter::DOWN),
        );

        info!("Toggle_Key: {:?}", Settings.Toggle_Key);

        Some(EventDispatcher {
            Settings: Settings,
            tx: tx,
            interception: interception,
            active: false,
            key_states: HashMap::new(),
            mouse_button_states: HashMap::new(),
        })
    }

    pub fn run(&mut self) {
        let mut strokes = [ic::Stroke::Keyboard {
            code: ic::ScanCode::Esc,
            state: ic::KeyState::empty(),
            information: 0,
        }; 10];

        loop {
            let device = self.interception.wait();

            let num_strokes = self.interception.receive(device, &mut strokes);
            let num_strokes = num_strokes as usize;

            for i in 0..num_strokes {
                let send = self.process_stroke(device, strokes[i]);
                if send {
                    self.interception.send(device, &strokes[i..i + 1]);
                }
            }
        }
    }

    fn process_stroke(&mut self, device: ic::Device, stroke: ic::Stroke) -> bool {
        match stroke {
            ic::Stroke::Keyboard {
                code,
                state,
                information: _,
            } => self.process_key(device, code, state.into()),

            ic::Stroke::Mouse {
                state,
                flags: _,
                rolling: _,
                x,
                y,
                information: _,
            } => {
                self.process_mouse_state(device, state);

                if !self.active {
                    return true;
                }

                if x != 0 || y != 0 {
                    self.tx.send(Event::MouseMove(x, y)).unwrap();
                }

                false
            }
        }
    }

    fn toggle_active(&mut self) {
        self.active = !self.active;

        if !self.active {
            self.tx.send(Event::Reset).unwrap();
            return;
        }

        for (&(device, code), &state) in self.key_states.iter() {
            if code == self.Settings.Toggle_Key || state == KeyState::Up {
                continue;
            }

            let stroke = [ic::Stroke::Keyboard {
                code: code,
                state: ic::KeyState::UP,
                information: 0,
            }];

            self.interception.send(device, &stroke);
        }

        for (&(device, button), &state) in self.mouse_button_states.iter() {
            if state == KeyState::Up {
                continue;
            }

            let button_flag = match button {
                MouseButton::Left => ic::MouseState::LEFT_BUTTON_UP,
                MouseButton::Right => ic::MouseState::RIGHT_BUTTON_UP,
                MouseButton::Middle => ic::MouseState::MIDDLE_BUTTON_UP,
                MouseButton::Button4 => ic::MouseState::BUTTON_4_UP,
                MouseButton::Button5 => ic::MouseState::BUTTON_5_UP,
            };

										
            let stroke = [ic::Stroke::Mouse {
                state: button_flag,
                flags: ic::MouseFlags::empty(),
                rolling: 0,
                x: 0,
                y: 0,
                information: 0,
            }];

            self.interception.send(device, &stroke);
			 
        }
    }

    fn process_key(&mut self, device: ic::Device, code: ic::ScanCode, state: KeyState) -> bool {
    let changed_state = match self.key_states.insert((device, code), state) {
        Some(old_state) => state != old_state,
        None => true,
    };

    if self.Settings.Excluded_Keys.contains(&code) {
        return true;
    }

    if code == self.Settings.Toggle_Key {
        if changed_state && state == KeyState::Down {
            self.toggle_active();
        }

        return false;
    }

    if self.active {
        if changed_state {
            self.tx.send(Event::Keyboard(code, state)).unwrap();
        }

        false
    } else {
        true
    }
}

    fn process_mouse_state(&mut self, device: ic::Device, state: ic::MouseState) {
        let table = [
            (
                ic::MouseState::LEFT_BUTTON_DOWN,
                ic::MouseState::LEFT_BUTTON_UP,
                MouseButton::Left,
            ),
            (
                ic::MouseState::RIGHT_BUTTON_DOWN,
                ic::MouseState::RIGHT_BUTTON_UP,
                MouseButton::Right,
            ),
            (
                ic::MouseState::MIDDLE_BUTTON_DOWN,
                ic::MouseState::MIDDLE_BUTTON_UP,
                MouseButton::Middle,
            ),
            (
                ic::MouseState::BUTTON_4_DOWN,
                ic::MouseState::BUTTON_4_UP,
                MouseButton::Button4,
            ),
            (
                ic::MouseState::BUTTON_5_DOWN,
                ic::MouseState::BUTTON_5_UP,
                MouseButton::Button5,
            ),		
        ];

        for &(flag_down, flag_up, button) in table.iter() {
            if (state.contains(flag_down) && state.contains(flag_up))
                || !state.intersects(flag_down | flag_up)
            {
                continue;
            }

            let key_state = match state.contains(flag_down) {
                true => KeyState::Down,
                false => KeyState::Up,
            };

            self.mouse_button_states.insert((device, button), key_state);

            if self.active {
                self.tx.send(Event::MouseButton(button, key_state)).unwrap();
            }
        }
    }
}
