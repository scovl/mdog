use crate::types::*;
use interception as ic;
use vigem::*;
use std::io::{self, Write};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bind {
    Keyboard(ic::ScanCode),
    Mouse(MouseButton),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ControllerAction {
    Button(ControllerButton),
    Analog(f64, f64),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    Sensitivity: f64,
    Parachute_Sensitivity: f64,
	binds: HashMap<Bind, ControllerAction>,
    Mouse_Smoothing_Level: u8,
    #[serde(skip)] 
    Mouse_Smoothing: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            Sensitivity: 1.0,
            Parachute_Sensitivity: f64::MAX,
            Mouse_Smoothing_Level: 5,
            Mouse_Smoothing: Duration::from_millis(5),
            binds: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn update_mouse_smoothing(&mut self) {
        let smoothing_millis = self.Mouse_Smoothing_Level.clamp(1, 10) as u64;
        self.Mouse_Smoothing = Duration::from_millis(smoothing_millis);
    }
}

pub struct EventHandler {
    Settings: Settings,
    rx: mpsc::Receiver<Event>,
    vigem: Vigem,
    target: Target,
    report: XUSBReport,
    mouse_samples: Arc<Mutex<VecDeque<(i32, i32, Instant)>>>,
    mouse_button_states: (KeyState, KeyState),
    parachute_Sensitivity_active: bool,
}

impl EventHandler {
    const ANALOG_MAX: f64 = -(i16::MIN as f64);
    const MAX_MOUSE_SAMPLES: usize = 100;

    pub fn new(rx: mpsc::Receiver<Event>, mut Settings: Settings) -> Result<Self, anyhow::Error> {
        let mut vigem = Vigem::new();
        vigem.connect()?;
        let mut target = Target::new(TargetType::Xbox360);
        vigem.target_add(&mut target)?;
		Settings.update_mouse_smoothing();

        info!("Controller Connected: {}", target.index());
        info!(
            "Sensitivity: {}, Mouse_Smoothing: {}",
            Settings.Sensitivity, Settings.Mouse_Smoothing.as_millis(),
        );
        info!("Parachute Mode: Press X");

        Ok(EventHandler {
            Settings,
            rx,
            vigem,
            target,
            report: XUSBReport::default(),
            mouse_samples: Arc::new(Mutex::new(VecDeque::new())),
            mouse_button_states: (KeyState::Up, KeyState::Up),
            parachute_Sensitivity_active: false,
        })
    }

    pub fn run(&mut self) -> Result<(), anyhow::Error> {
        let mut w = false;
        let mut a = false;
        let mut s = false;
        let mut d = false;
        let (mouse_tx, mouse_rx) = mpsc::channel();
        let mouse_samples_thread = Arc::clone(&self.mouse_samples);
        thread::spawn(move || {
            while let Ok((x, y)) = mouse_rx.recv() {
                let mut samples = mouse_samples_thread.lock().unwrap();
                let now = Instant::now();
                samples.push_back((x, y, now));
                if samples.len() > EventHandler::MAX_MOUSE_SAMPLES {
                    samples.pop_front();
                }
            }
        });

        loop {
            let timeout = Duration::from_micros(10);
            match self.rx.recv_timeout(timeout) {
                Ok(event) => match event {
                    Event::MouseMove(x, y) => {
                        let _ = mouse_tx.send((x, y));
                    }
                    Event::MouseButton(button, state) => {
                        if button == MouseButton::Left {
                            self.mouse_button_states.0 = state;
                        }
                        if button == MouseButton::Right {
                            self.mouse_button_states.1 = state;
                        }
                        self.handle_bind(Bind::Mouse(button), state);
                    }
                    Event::Keyboard(scancode, state) => {
                        self.handle_bind(Bind::Keyboard(scancode), state);
                        match (state, scancode) {
                            (KeyState::Up, ic::ScanCode::W) => w = false,
                            (KeyState::Up, ic::ScanCode::A) => a = false,
                            (KeyState::Up, ic::ScanCode::S) => s = false,
                            (KeyState::Up, ic::ScanCode::D) => d = false,
                            (KeyState::Down, ic::ScanCode::W) => w = true,
                            (KeyState::Down, ic::ScanCode::A) => a = true,
                            (KeyState::Down, ic::ScanCode::S) => s = true,
                            (KeyState::Down, ic::ScanCode::D) => d = true,
                            _ => {}
                        }
                        self.update_movement(w, a, s, d);
                    }
                    Event::Reset => {
                        self.mouse_button_states = (KeyState::Up, KeyState::Up);
                        self.report = XUSBReport::default();
                    }
                },
                Err(_) => {
                }
            }
            self.update_analog();
            self.vigem.update(&self.target, &self.report)?;
        }
    }

    fn handle_bind(&mut self, bind: Bind, state: KeyState) {
        if let Some(action) = self.Settings.binds.get(&bind) {
            match action {
                ControllerAction::Button(controller_button) => match controller_button {
                    ControllerButton::LeftTrigger => {
                        self.report.b_left_trigger =
                            if state == KeyState::Down { u8::MAX } else { 0 }
                    }
                    ControllerButton::RightTrigger => {
                        self.report.b_right_trigger =
                            if state == KeyState::Down { u8::MAX } else { 0 }
                    }
                    button => {
                        let button_flag = XButton::from_bits(*button as u16).unwrap();
                        if state == KeyState::Down {
                            self.report.w_buttons |= button_flag;
                        } else {
                            self.report.w_buttons &= !button_flag;
                        }
                    }
                },
                ControllerAction::Analog(_, _) => {}
            }
        }
        if let Bind::Keyboard(ic::ScanCode::X) = bind {
            if state == KeyState::Down {
                self.parachute_Sensitivity_active = !self.parachute_Sensitivity_active;
                print!(
                    "\rParachute Mode {} \x1b[K",
                    if self.parachute_Sensitivity_active {
                        "Active"
                    } else {
                        "Deactivate"
                    }
                );
                io::stdout().flush().unwrap();
            }
        }
    }

fn update_analog(&mut self) {
    let now = Instant::now();
    let smoothing = self.Settings.Mouse_Smoothing;
    let mut mouse_vel = (0.0, 0.0);

    if let Ok(mut samples) = self.mouse_samples.lock() {
        while samples.len() > 5 {
            samples.pop_front();
        }
		
        while let Some(sample) = samples.front() {
            if now - sample.2 > smoothing {
                samples.pop_front();
            } else {
                break;
            }
        }

        for &(x, y, _) in samples.iter() {
            mouse_vel.0 += x as f64;
            mouse_vel.1 += y as f64;
        }
    }

    let Sensitivity = if self.parachute_Sensitivity_active {
        self.Settings.Parachute_Sensitivity
    } else {
        self.Settings.Sensitivity
    };

    let multiplier = Sensitivity / (1e4 * smoothing.as_secs_f64().max(1e-4));
    let analog_x = mouse_vel.0 * multiplier;
    let analog_y = -mouse_vel.1 * multiplier;
	self.set_analog_linear(analog_x, analog_y);
    }

    fn update_movement(&mut self, w: bool, a: bool, s: bool, d: bool) {
        self.report.s_thumb_ly = if w == s { 0 } else { if w { i16::MAX } else { i16::MIN }};
        self.report.s_thumb_lx = if a == d { 0 } else { if d { i16::MAX } else { i16::MIN }};
    }

    fn set_analog_linear(&mut self, x: f64, y: f64) {
        let magnitude = (x.powi(2) + y.powi(2)).sqrt();
        let threshold = 1.0;

        if magnitude <= threshold {
            self.report.s_thumb_rx = (x * Self::ANALOG_MAX) as i16;
            self.report.s_thumb_ry = (y * Self::ANALOG_MAX) as i16;
        } else {
            let angle = y.atan2(x);
            let new_radius = magnitude.min(1.0);

            self.report.s_thumb_rx = (angle.cos() * new_radius * Self::ANALOG_MAX) as i16;
            self.report.s_thumb_ry = (angle.sin() * new_radius * Self::ANALOG_MAX) as i16;
        }
    }
}