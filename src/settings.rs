use std::sync::Mutex;

use crate::evetrade::EvetradeError;

pub struct Settings {
    log_level: log::LevelFilter,
    update_universe_data: bool,
    percentage_threshold: f32,
    ship_cargo_volume: f32,
    max_jumps: u32,
    initial_capital: f32,
    security_threshold: f32,
    pair_profit_threshold: f32,
    profit_goal: f32,
    jump_window: f32,
    similarity_threshold: f32,
    manual_download: bool,
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
            pair_profit_threshold: 0.0,
            profit_goal: 0.0,
            jump_window: 0.0,
            similarity_threshold: 0.0,
            manual_download = false,
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

        if config.get("route").is_none() {
            println!("Settings::parse_config() : no 'route' section in the config.");
            return Err(EvetradeError::ConfigError);
        }

        if config.get("advanced").is_none() {
            println!("Settings::parse_config() : no 'advanced' section in the config.");
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

        if percentage_treshold <= 0.0 {
            println!("Settings::parse_config() : invalid threshold value.");
            return Err(EvetradeError::ConfigError);
        }

        let ship_cargo_volume =
            config["user"]["ship_cargo_volume"]
                .as_float()
                .ok_or_else(|| {
                    println!(
                        "Settings::parse_config() : failed to parse 'user->ship_cargo_volume'."
                    );
                    EvetradeError::ConfigError
                })?;

        let max_jumps = config["route"]["max_jumps"].as_integer().ok_or_else(|| {
            println!("Settings::parse_config() : failed to parse 'route->max_jumps'.");
            EvetradeError::ConfigError
        })?;

        let initial_capital = config["user"]["initial_capital"]
            .as_float()
            .ok_or_else(|| {
                println!("Settings::parse_config() : failed to parse 'user->initial_capital'.");
                EvetradeError::ConfigError
            })?;

        let security_threshold = config["route"]["security_threshold"]
            .as_float()
            .ok_or_else(|| {
                println!("Settings::parse_config() : failed to parse 'route->security_threshold'.");
                EvetradeError::ConfigError
            })?;

        let pair_profit_threshold = config["thresholds"]["pair_profit_threshold"]
            .as_float()
            .ok_or_else(|| {
                println!(
                    "Settings::parse_config() : failed to parse 'thresholds->pair_profit_threshold'."
                );
                EvetradeError::ConfigError
            })?;

        let profit_goal = config["user"]["profit_goal"].as_float().ok_or_else(|| {
            println!("Settings::parse_config() : failed to parse 'user->profit_goal'.");
            EvetradeError::ConfigError
        })?;

        if profit_goal <= 0.0 {
            println!("Settings::parse_config() : invalid profit goal value.");
            return Err(EvetradeError::ConfigError);
        }

        let jump_window = config["advanced"]["jump_window"]
            .as_float()
            .ok_or_else(|| {
                println!("Settings::parse_config() : failed to parse 'advanced->jump_window'.");
                EvetradeError::ConfigError
            })?;

        if jump_window <= 0.0 {
            println!("Settings::parse_config() : jump window is too low.");
            return Err(EvetradeError::ConfigError);
        }

        let similarity_threshold = config["advanced"]["similarity_threshold"]
            .as_float()
            .ok_or_else(|| {
                println!(
                    "Settings::parse_config() : failed to parse 'advanced->similarity_threshold'."
                );
                EvetradeError::ConfigError
            })?;

        let manual_download = config["advanced"]["manual_download"]
        .as_bool()
        .ok_or_else(|| {
            println!(
                "Settings::parse_config() : failed to parse 'advanced->manual_download'."
            );
            EvetradeError::ConfigError
        })?;

        let mut settings = SETTINGS.lock().unwrap();

        settings.log_level = log_level;
        settings.update_universe_data = update_universe_data;
        settings.percentage_threshold = percentage_treshold as f32;
        settings.ship_cargo_volume = ship_cargo_volume as f32;
        settings.max_jumps = max_jumps as u32;
        settings.initial_capital = initial_capital as f32;
        settings.security_threshold = security_threshold as f32;
        settings.pair_profit_threshold = pair_profit_threshold as f32;
        settings.profit_goal = profit_goal as f32;
        settings.jump_window = jump_window as f32;
        settings.similarity_threshold = similarity_threshold as f32;
        settings.manual_download = manual_download;

        Ok(())
    }

    pub fn get_similarity_threshold() -> f32 {
        SETTINGS.lock().unwrap().similarity_threshold
    }

    pub fn get_manual_download() -> f32 {
        SETTINGS.lock().unwrap().manual_download
    }

    pub fn get_jump_window() -> f32 {
        SETTINGS.lock().unwrap().jump_window
    }

    pub fn get_profit_goal() -> f32 {
        SETTINGS.lock().unwrap().profit_goal
    }

    pub fn get_pair_profit_threshold() -> f32 {
        SETTINGS.lock().unwrap().pair_profit_threshold
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

    pub fn set_percentage_threshold(threshold: f32) {
        SETTINGS.lock().unwrap().percentage_threshold = threshold;
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

    pub fn set_pair_profit_threshold(threshold: f32) {
        SETTINGS.lock().unwrap().pair_profit_threshold = threshold;
    }

    pub fn set_profit_goal(goal: f32) {
        SETTINGS.lock().unwrap().profit_goal = goal;
    }

    pub fn set_jump_window(window: f32) {
        SETTINGS.lock().unwrap().jump_window = window;
    }
}

lazy_static! {
    static ref SETTINGS: Mutex<Settings> = Mutex::new(Settings::new());
}
