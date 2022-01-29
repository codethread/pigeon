pub use log::{debug, error, info, warn};
use simplelog::*;
use std::fs::File;

/// Setup logger, must be called early
pub fn init_logger() {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Debug,
        Config::default(),
        File::create("log.txt").unwrap(),
    )])
    .unwrap();
    info!("logger created and writing to: log.txt")
}
