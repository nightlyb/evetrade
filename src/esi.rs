use bincode;
use bzip2::read::BzDecoder;
use log::{debug, error, info};
use reqwest;
use serde;
use serde_yaml;
use std::collections::HashMap;
use std::io::{Read, Write};
use tar::Archive;
use xz2::read::XzDecoder;

use crate::settings::Settings;
use crate::types::{Order, OrderGroup, Stargate, System, Type, Vector3};
use crate::urls;

// astronomical units to meters
const AU: i128 = 149_597_870_700;

// {
//         let mut settings = SETTINGS.lock().unwrap();
//         settings.set_field1("new_value".to_string());
//         settings.set_field2(42);
//     }

#[derive(Debug)]
pub enum ESIError {
    RequestError,
    IoError(std::io::Error),
    InvalidData,
}

pub struct ESI {
    pub orders: std::collections::HashMap<u32, OrderGroup>,
    pub systems: HashMap<u32, System>,
    pub types: HashMap<u32, Type>,
    //pub mean_jump_distance: f64,
}

impl ESI {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            systems: HashMap::new(),
            types: HashMap::new(),
            //mean_jump_distance: 0.0,
        }
    }

    pub fn get_all_data(&mut self) -> Result<(), ESIError> {
        let path_exists = std::path::Path::new(".evecache/").exists();

        if !path_exists || Settings::get_update_universe_data() {
            info!("Cache directory does not exist or updating universe data was explicitly requested by the user.");

            if let Err(err) = self.fetch_universe_data() {
                error!("Failed to fetch universe data!");
                return Err(err);
            }

            self.fetch_systems()?;
            self.fetch_types()?;

            ESI::save(&self.systems, ".evecache/systems.bin")?;
            ESI::save(&self.types, ".evecache/types.bin")?;
        } else {
            info!("Using cached systems and types data.");

            self.systems = ESI::load(".evecache/systems.bin")?;
            self.types = ESI::load(".evecache/types.bin")?;
        }

        let orders_path = std::path::Path::new(".evecache/orders.bin");
        if !orders_path.exists() {
            info!("Cached orders were not found, fetching...");

            self.fetch_orders()?;

            ESI::save(&self.orders, ".evecache/orders.bin")?;
        } else {
            let modified_time = orders_path.metadata().unwrap().modified().unwrap();
            let fifteen_minutes_ago =
                std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 15);

            if modified_time < fifteen_minutes_ago {
                info!("Orders are outdated, updating...");

                self.fetch_orders()?;

                ESI::save(&self.orders, ".evecache/orders.bin")?;
            } else {
                info!("Using cached orders data.");

                self.orders = ESI::load(".evecache/orders.bin")?;
            }
        }

        //self.mean_jump_distance = self.calculate_mean_jump_distance();

        Ok(())
    }

    // fn calculate_mean_jump_distance(&mut self) -> i128 {
    //     let mut total_distance = 0.0;
    //     let mut total_jumps = 0;

    //     for system in self.systems.values() {
    //         for stargate in &system.stargates {
    //             let destination_system = self.systems.get(&stargate.destination).unwrap();
    //             total_distance += system.position.distance(&destination_system.position);
    //             total_jumps += 1;
    //         }
    //     }

    //     total_distance / total_jumps as f64;
    //     0
    // }

    fn fetch_universe_data(&mut self) -> Result<(), ESIError> {
        info!("Updating universe data...");

        let response = reqwest::blocking::get(urls::get_esi_scrape_url()).map_err(|err| {
            error!("Failed to perform an API call! \n\tError: {}", err);
            ESIError::RequestError
        })?;

        let mut decompressor = XzDecoder::new(response);
        let mut data_buffer = Vec::new();

        info!("Decompressing universe data...");
        decompressor.read_to_end(&mut data_buffer).map_err(|err| {
            error!("Failed to read decompressed data! \n\tError: {}", err);
            ESIError::InvalidData
        })?;

        let cursor = std::io::Cursor::new(data_buffer);
        let mut archive = Archive::new(cursor);

        info!("Extracting universe data...");
        archive.unpack(".evecache").map_err(|err| {
            error!("Failed to unpack archive! \n\tError: {}", err);
            ESIError::InvalidData
        })?;

        Ok(())
    }

    fn get_stargates() -> Result<HashMap<u32, Vec<u32>>, ESIError> {
        let data = std::fs::read_to_string(
            ".evecache/eve-ref-esi-scrape/data/tranquility/universe/stargates.en-us.yaml",
        )
        .map_err(|err| {
            error!("Failed to read stargates data! \n\tError: {}", err);
            ESIError::IoError(err)
        })?;

        let stargates: HashMap<String, StargateData> =
            serde_yaml::from_str(&data).map_err(|err| {
                error!("Failed to parse stargates data! \n\tError: {}", err);
                ESIError::InvalidData
            })?;

        let mut stargate_map = HashMap::new();
        for (_, value) in stargates {
            let system_id = value.system_id;
            let destination_system_id = value.destination.system_id;

            stargate_map
                .entry(system_id)
                .or_insert_with(Vec::new)
                .push(destination_system_id);
        }

        Ok(stargate_map)
    }

    fn fetch_systems(&mut self) -> Result<(), ESIError> {
        let stargates = ESI::get_stargates()?;
        let data = std::fs::read_to_string(
            ".evecache/eve-ref-esi-scrape/data/tranquility/universe/systems.en-us.yaml",
        )
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                error!("Data not found. Delete '.evecache' folder and try again.");
            }
            ESIError::IoError(err)
        })?;

        let systems: HashMap<String, SystemData> = serde_yaml::from_str(&data).map_err(|err| {
            error!("Failed to parse systems data! \n\tError: {}", err);
            ESIError::InvalidData
        })?;

        info!("Parsing system data...");
        for (key, value) in &systems {
            let system_id = key.parse::<u32>().unwrap();
            let name = &value.name;
            let security_status = ((value.security_status * 10.0).round() / 10.0) as f32;

            let system_position = SystemPosition {
                x: value.position.x,
                y: value.position.y,
                z: value.position.z,
            };

            let mut system_stargates = Vec::new();
            if let Some(stargate_destinations) = stargates.get(&system_id) {
                for &stargate_destination in stargate_destinations {
                    let mut destination_security = self
                        .systems
                        .get(&stargate_destination)
                        .map_or(f32::INFINITY, |system| system.security_status);

                    if destination_security == f32::INFINITY {
                        if let Some(dest_system) = systems.get(&stargate_destination.to_string()) {
                            destination_security = dest_system.security_status;
                        }
                    }

                    let weight = (1.0 + destination_security) / 2.0;

                    system_stargates.push(Stargate {
                        origin: system_id,
                        destination: stargate_destination,
                        weight,
                    });
                }
            }

            if system_stargates.is_empty() {
                continue;
            }

            let position = Vector3 {
                x: (system_position.x / AU) as f64,
                y: (system_position.y / AU) as f64,
                z: (system_position.z / AU) as f64,
            };

            self.systems.insert(
                system_id,
                System {
                    id: system_id,
                    name: name.to_string(),
                    security_status,
                    stargates: system_stargates,
                    position,
                },
            );
        }

        let mut scale = 0.0;
        let mut min_scale = f64::INFINITY;
        for system in self.systems.values() {
            for stargate in &system.stargates {
                let destination_system = self.systems.get(&stargate.destination).unwrap();
                let distance = system.position.distance(&destination_system.position);
                if distance > scale {
                    scale = distance;
                }
                if distance == 0.0 {
                    println!("distance 0");
                } else if distance < min_scale {
                    min_scale = distance;
                }
            }
        }

        Ok(())
    }

    fn fetch_types(&mut self) -> Result<(), ESIError> {
        let data = std::fs::read_to_string(
            ".evecache/eve-ref-esi-scrape/data/tranquility/universe/types.en-us.yaml",
        )
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                error!("Data not found. Delete '.evecache' folder and try again.");
            }
            ESIError::IoError(err)
        })?;

        let types: HashMap<String, TypeData> = serde_yaml::from_str(&data).map_err(|err| {
            error!("Failed to parse types data! \n\tError: {}", err);
            ESIError::InvalidData
        })?;

        info!("Parsing type data...");
        for (key, value) in types {
            if !value.published {
                continue;
            }

            let type_id = key.parse::<u32>().unwrap();
            self.types.insert(
                type_id,
                Type {
                    type_id,
                    group_id: value.group_id,
                    name: value.name,
                    volume: value.packaged_volume as f32,
                },
            );
        }

        Ok(())
    }

    fn fetch_orders(&mut self) -> Result<(), ESIError> {
        let response = reqwest::blocking::get(urls::get_market_data_url()).map_err(|err| {
            error!("Failed to perform an API call! \n\tError: {}", err);
            ESIError::RequestError
        })?;

        let mut decompressor = BzDecoder::new(response);
        let mut data_buffer = Vec::new();

        info!("Decompressing orders...");
        decompressor.read_to_end(&mut data_buffer).map_err(|err| {
            error!("Failed to read decompressed data! \n\tError: {}", err);
            ESIError::InvalidData
        })?;

        let csv_data = std::io::Cursor::new(data_buffer);
        let mut reader = csv::Reader::from_reader(csv_data);

        info!("Parsing order data...");
        for result in reader.records() {
            let record = result.map_err(|err| {
                error!("Failed to parse order data! \n\tError: {}", err);
                ESIError::InvalidData
            })?;

            let type_id: u32 = record.get(9).unwrap_or("0").parse().unwrap_or(0);

            let order_type = match self.types.get(&type_id) {
                Some(order_type) => order_type,
                None => continue,
            };

            let order = Order {
                is_buy_order: record.get(1).unwrap_or("false").parse().unwrap_or(false),
                price: record.get(6).unwrap_or("0.0").parse().unwrap_or(0.0),
                station_id: record.get(13).unwrap_or("0").parse().unwrap_or(0),
                system_id: record.get(8).unwrap_or("0").parse().unwrap_or(0),
                region_id: record.get(14).unwrap_or("0").parse().unwrap_or(0),
                volume: record.get(10).unwrap_or("0.0").parse().unwrap_or(0),
                order_type: order_type.clone(),
                type_id,
            };

            if order.station_id == 0 {
                continue;
            }

            self.orders
                .entry(type_id)
                .or_insert_with(OrderGroup::new)
                .add_order(order);
        }

        Ok(())
    }

    pub fn save<T: serde::Serialize>(data: &T, path: &str) -> Result<(), ESIError> {
        debug!("Trying to save... \n\tPath: {}", path);

        let encoded = bincode::serialize(data).map_err(|err| {
            error!("Failed to serialize data! \n\tError: {}", err);
            ESIError::InvalidData
        })?;

        let mut file = std::fs::File::create(path).map_err(|err| {
            error!(
                "Failed to create file! \n\tPath: {}\n\tError: {}",
                path, err
            );
            ESIError::IoError(err)
        })?;

        file.write_all(&encoded).map_err(|err| {
            error!("Failed to write data! \n\tPath: {}\n\tError: {}", path, err);
            ESIError::IoError(err)
        })?;

        debug!("Save successful!");

        Ok(())
    }

    pub fn load<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, ESIError> {
        debug!("Trying to load... \n\tPath: {}", path);
        let mut file = std::fs::File::open(path).map_err(|err| {
            error!("Failed to open file! \n\tPath: {}\n\tError: {}", path, err);
            ESIError::IoError(err)
        })?;

        let mut encoded = Vec::new();
        file.read_to_end(&mut encoded).map_err(|err| {
            error!("Failed to read data! \n\tError: {}", err);
            ESIError::IoError(err)
        })?;

        let data = bincode::deserialize(&encoded).map_err(|err| {
            error!("Failed to deserialize data! \n\tError: {}", err);
            ESIError::InvalidData
        })?;

        debug!("Load successful!");

        Ok(data)
    }
}

impl std::fmt::Display for ESIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ESIError::RequestError => write!(f, "Request error"),
            ESIError::IoError(err) => write!(f, "IO error: {}", err),
            ESIError::InvalidData => write!(f, "Invalid data"),
        }
    }
}

impl From<std::io::Error> for ESIError {
    fn from(err: std::io::Error) -> ESIError {
        ESIError::IoError(err)
    }
}

// These are required for easier serde_yaml deserialization.
#[derive(Debug, serde::Deserialize)]
struct SystemData {
    name: String,
    security_status: f32,
    position: SystemPosition,
}

#[derive(Debug, serde::Deserialize)]
struct SystemPosition {
    x: i128,
    y: i128,
    z: i128,
}

#[derive(Debug, serde::Deserialize)]
struct StargateDestination {
    system_id: u32,
}

#[derive(Debug, serde::Deserialize)]
struct StargateData {
    destination: StargateDestination,
    system_id: u32,
}

#[derive(Debug, serde::Deserialize)]
struct TypeData {
    group_id: u32,
    name: String,
    packaged_volume: f64,
    published: bool,
}
