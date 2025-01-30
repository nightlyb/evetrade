use std::sync::Mutex;

pub struct Settings {
    log_level: log::Level,
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
            log_level: log::Level::Debug,
            update_universe_data: false,
            percentage_threshold: 10.0,
            ship_cargo_volume: 6300.0,
            max_jumps: 100,
            initial_capital: 50000000.0,
            security_threshold: -1.0,
        }
    }

    pub fn get_level() -> log::Level {
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

    pub fn set_level(level: log::Level) {
        SETTINGS.lock().unwrap().log_level = level;
    }
}

lazy_static! {
    static ref SETTINGS: Mutex<Settings> = Mutex::new(Settings::new());
}
