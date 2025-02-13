#![allow(non_snake_case)]

use log::{info, error};
use clap::Parser;
use colored::*;
use std::{
    fs::File,
    io::Write,
    path::Path,
    sync::mpsc,
    thread,
};
use thread_priority::{set_current_thread_priority, ThreadPriority};

mod event_dispatcher;
mod event_handler;
mod types;

use event_dispatcher::EventDispatcher;
use event_handler::EventHandler;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[clap(version = "0.1.2")]
struct Opts {
    #[clap(short, long, default_value = "settings.ron")]
    settings_file: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Settings {
    event_dispatcher: event_dispatcher::Settings,
    event_handler: event_handler::Settings,
}

fn load_settings<P: AsRef<Path>>(path: P) -> Settings {
    let path_str = path.as_ref().to_string_lossy();

    match File::open(&path).map(ron::de::from_reader) {
        Ok(Ok(settings)) => {
            info!("Load Configuration: \"{}\"", path_str);
            settings
        }
        Err(error) => {
            error!("Configuration Not Found \"{}\": {}", path_str, error);
            Settings::default()
        }
        Ok(Err(error)) => {
            error!("Cannot Process Configuration \"{}\": {}", path_str, error);
            Settings::default()
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger();
    
    let opts = Opts::parse();
    let Settings {
        event_dispatcher: dispatcher_settings,
        event_handler: handler_settings,
    } = load_settings(opts.settings_file);

    let (tx, rx) = mpsc::channel();

    let event_handler_thread = thread::spawn(move || {
        if let Err(e) = set_current_thread_priority(ThreadPriority::Max) {
            error!("Failed to set thread priority: {}", e);
        }
        
        match EventHandler::new(rx, handler_settings) {
            Ok(mut handler) => {
                if let Err(e) = handler.run() {
                    error!("Event handler error: {}", e);
                }
            }
            Err(e) => error!("Failed to create event handler: {}", e),
        }
    });

    if let Some(mut dispatcher) = EventDispatcher::new(tx, dispatcher_settings) {
        dispatcher.run();
    } else {
        error!("Failed to create event dispatcher");
    }

    event_handler_thread.join().expect("Event handler thread panicked");
    Ok(())
}

fn setup_logger() {
    env_logger::Builder::from_env(env_logger::Env::default()
        .filter_or(env_logger::DEFAULT_FILTER_ENV, "info"))
        .format(|buf, record| {
            let level = match record.level() {
                log::Level::Info => record.level().to_string().red(),
                _ => record.level().to_string().normal(),
            };
            writeln!(buf, "[{}] {}", level, record.args())
        })
        .format_timestamp(None)
        .init();
}