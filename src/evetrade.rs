use chrono::Local;
use env_logger::Builder;
use log::{error, info, Level, LevelFilter};
use std::io::Write;

use crate::esi;
use crate::processor::OrderProcessor;
use crate::route::Route;
use crate::settings::Settings;

#[derive(Debug)]
pub enum EvetradeError {
    ESIError,
    IOError,
    ConfigError,
}

pub struct Evetrade {
    esi: esi::ESI,
    is_initialized: bool,
    routes: Vec<Route>,
}

impl Evetrade {
    pub fn new() -> Self {
        Self {
            is_initialized: false,
            esi: esi::ESI::new(),
            routes: Vec::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), EvetradeError> {
        println!("initializing logger...");
        Settings::parse_config().map_err(|err| {
            println!("Evetrade::init() : error in the config: {}", err);
            EvetradeError::ConfigError
        })?;

        Builder::new()
            .format(|buf, record| {
                let now = Local::now();
                let time = now.format("%H:%M:%S"); // HOUR:MINUTE:SECOND

                let millis_full = now.format("%.3f").to_string();
                let millis = millis_full.trim_start_matches('.');

                let mut time_style = buf.style();
                time_style
                    .set_color(env_logger::fmt::Color::Blue)
                    .set_bold(true);

                let mut level_style = buf.style();
                match record.level() {
                    Level::Error => level_style
                        .set_color(env_logger::fmt::Color::Red)
                        .set_bold(true)
                        .set_bg(env_logger::fmt::Color::Black)
                        .set_bold(true),
                    Level::Warn => level_style
                        .set_color(env_logger::fmt::Color::Yellow)
                        .set_bold(true),
                    Level::Info => level_style
                        .set_color(env_logger::fmt::Color::Green)
                        .set_bold(true),
                    Level::Debug => level_style.set_color(env_logger::fmt::Color::Cyan),
                    Level::Trace => level_style.set_color(env_logger::fmt::Color::Magenta),
                };

                let mut target_style = buf.style();
                target_style
                    .set_color(env_logger::fmt::Color::White)
                    .set_bold(true);

                writeln!(
                    buf,
                    "[{}.{} {:<5} {:<20}] {}",
                    time_style.value(time),
                    millis,
                    level_style.value(record.level()),
                    target_style.value(record.target()),
                    record.args()
                )
            })
            .filter_level(Settings::get_log_level())
            .init();

        info!("Logger initialized successfully!");

        if self.esi.get_all_data().is_err() {
            error!("Failed to fetch all required data! Shutting down...");
            return Err(EvetradeError::ESIError);
        }

        self.is_initialized = true;
        Ok(())
    }

    pub fn compute(&mut self) -> Result<(), EvetradeError> {
        info!("Computing routes...");
        let mut processor = OrderProcessor::new(
            &mut self.esi.orders,
            &mut self.esi.systems,
            &self.esi.types,
            //self.esi.mean_jump_distance,
        );

        self.routes = processor.compute();

        info!("Sorting routes...");
        Route::sort_routes(&mut self.routes);

        Ok(())
    }

    pub fn display_and_save(&mut self) -> Result<(), EvetradeError> {
        info!("Displaying routes...");

        for route in &mut self.routes {
            println!("{}", route.represent());
        }

        info!("Saving routes...");

        let mut file = std::fs::File::create("results.txt").map_err(|err| {
            error!("Failed to create file: {}", err);
            EvetradeError::IOError
        })?;

        for route in &mut self.routes {
            writeln!(file, "{}", route.represent()).map_err(|err| {
                error!("Failed to write to file: {}", err);
                EvetradeError::IOError
            })?;
        }

        Ok(())
    }
}

impl std::fmt::Display for EvetradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvetradeError::ESIError => write!(f, "Failed to perform API requests!"),
            EvetradeError::IOError => write!(f, "Failed to save routes!"),
            EvetradeError::ConfigError => write!(f, "Failed to load the configuration file!"),
        }
    }
}

impl std::error::Error for EvetradeError {}
