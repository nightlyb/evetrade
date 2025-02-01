use std::{fmt::Debug, sync::Mutex};

use crate::evetrade::EvetradeError;

pub struct Settings {
    log_level: log::LevelFilter,
    update_universe_data: bool,
    percentage_threshold: f32,
    ship_cargo_volume: f32,
    max_jumps: u32,
    initial_capital: f32,
    security_threshold: f32,
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            log_level: log::LevelFilter::Trace,
            update_universe_data: false,
            percentage_threshold: 0.0,
            ship_cargo_volume: 0.0,
            max_jumps: 0,
            initial_capital: 0.0,
            security_threshold: 0.0,
        }
    }

    pub fn parse_config() -> Result<(), EvetradeError> {
        let config_file = std::fs::read_to_string("config.toml").map_err(|e| {
            println!("Error reading config file: {}", e);
            EvetradeError::IOError
        })?;

        let config: toml::Value = toml::from_str(&config_file).map_err(|e| {
            println!("Error parsing config file: {}", e);
            EvetradeError::ConfigError
        })?;

        if config.get("settings").is_none() {
            println!("Settings::parse_config() : no 'settings' section in the config.");
            return Err(EvetradeError::ConfigError);
        }

        if config.get("thresholds").is_none() {
            println!("Settings::parse_config() : no 'thresholds' section in the config.");
            return Err(EvetradeError::ConfigError);
        }

        if config.get("user").is_none() {
            println!("Settings::parse_config() : no 'user' section in the config.");
            return Err(EvetradeError::ConfigError);
        }

        let log_level = config["settings"]["log_level"].as_str().unwrap_or_else(|| {
            println!("Settings::parse_config() : failed to parse 'Settings->log_level' in the config. Using default.");
            "debug"
        });

        let log_level = match log_level.to_lowercase().as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Debug,
        };

        let update_universe_data = config["settings"]["update_universe_data"]
            .as_bool()
            .ok_or_else(|| {
                println!(
                    "Settings::parse_config() : failed to parse 'settings->update_universe_data'."
                );
                EvetradeError::ConfigError
            })?;

        let percentage_treshold = config["thresholds"]["percentage_threshold"]
            .as_float()
            .ok_or_else(|| {
                println!(
                    "Settings::parse_config() : failed to parse 'thresholds->percentage_treshold'."
                );
                EvetradeError::ConfigError
            })?;

        let ship_cargo_volume =
            config["user"]["ship_cargo_volume"]
                .as_float()
                .ok_or_else(|| {
                    println!(
                        "Settings::parse_config() : failed to parse 'user->ship_cargo_volume'."
                    );
                    EvetradeError::ConfigError
                })?;

        let max_jumps = config["thresholds"]["max_jumps"]
            .as_integer()
            .ok_or_else(|| {
                println!("Settings::parse_config() : failed to parse 'thresholds->max_jumps'.");
                EvetradeError::ConfigError
            })?;

        let initial_capital = config["user"]["initial_capital"]
            .as_float()
            .ok_or_else(|| {
                println!("Settings::parse_config() : failed to parse 'user->initial_capital'.");
                EvetradeError::ConfigError
            })?;

        let security_threshold = config["thresholds"]["security_threshold"]
            .as_float()
            .ok_or_else(|| {
                println!(
                    "Settings::parse_config() : failed to parse 'thresholds->security_threshold'."
                );
                EvetradeError::ConfigError
            })?;

        Settings::set_update_universe_data(update_universe_data);
        Settings::set_percentage_threshold(percentage_treshold as f32)?;
        Settings::set_ship_cargo_volume(ship_cargo_volume as f32);
        Settings::set_max_jumps(max_jumps as u32);
        Settings::set_log_level(log_level);
        Settings::set_initial_capital(initial_capital as f32);
        Settings::set_security_threshold(security_threshold as f32);

        Ok(())
    }

    pub fn get_log_level() -> log::LevelFilter {
        SETTINGS.lock().unwrap().log_level
    }

    pub fn get_update_universe_data() -> bool {
        SETTINGS.lock().unwrap().update_universe_data
    }

    pub fn get_percentage_threshold() -> f32 {
        SETTINGS.lock().unwrap().percentage_threshold
    }

    pub fn get_max_jumps() -> u32 {
        SETTINGS.lock().unwrap().max_jumps
    }

    pub fn get_ship_cargo_volume() -> f32 {
        SETTINGS.lock().unwrap().ship_cargo_volume
    }

    pub fn get_initial_capital() -> f32 {
        SETTINGS.lock().unwrap().initial_capital
    }

    pub fn set_log_level(level: log::LevelFilter) {
        SETTINGS.lock().unwrap().log_level = level;
    }

    pub fn set_update_universe_data(update: bool) {
        SETTINGS.lock().unwrap().update_universe_data = update;
    }

    pub fn set_percentage_threshold(threshold: f32) -> Result<(), EvetradeError> {
        if threshold <= 0.0 {
            println!("Settings::set_percentage_threshold() : invalid threshold value.");
            return Err(EvetradeError::ConfigError);
        }
        SETTINGS.lock().unwrap().percentage_threshold = threshold;
        Ok(())
    }

    pub fn set_max_jumps(jumps: u32) {
        SETTINGS.lock().unwrap().max_jumps = jumps;
    }

    pub fn set_ship_cargo_volume(volume: f32) {
        SETTINGS.lock().unwrap().ship_cargo_volume = volume;
    }

    pub fn set_initial_capital(capital: f32) {
        SETTINGS.lock().unwrap().initial_capital = capital;
    }

    pub fn set_security_threshold(threshold: f32) {
        SETTINGS.lock().unwrap().security_threshold = threshold;
    }
}

lazy_static! {
    static ref SETTINGS: Mutex<Settings> = Mutex::new(Settings::new());
}
