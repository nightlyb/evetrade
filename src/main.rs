mod esi;
mod evetrade;
mod kdtree;
mod pathfinder;
mod processor;
mod route;
mod settings;
mod types;
mod urls;

#[macro_use]
extern crate lazy_static;

use log::{error, info};

use evetrade::Evetrade;

fn main() {
    println!("initializing logger...");

    let mut et = Evetrade::new();

    if let Err(e) = et.init() {
        error!("Failed to initialize Evetrade: {:?}", e);
        return;
    };

    if let Err(e) = et.compute() {
        error!("Evetrade failed to compute routes: {:?}", e);
        return;
    }

    if let Err(e) = et.display_and_save() {
        error!("Evetrade failed to display and save routes: {:?}", e);
        return;
    }

    info!("Done!");
}
