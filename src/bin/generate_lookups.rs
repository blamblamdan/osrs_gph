use std::{path::PathBuf, collections::HashMap};

use osrs_gph::{convenience, logging::{LogConfig, LogFileIO}, errors::Custom, log_panic};
use slog::{debug, Level, warn};
use sloggers::types::Format;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct MappingItem {
    #[serde(default)]
    highalch: i32,
    members: bool,
    name: String,
    examine: String,
    id: i32,
    value: i32,
    icon: String,
    #[serde(default)]
    lowalch: i32
}

/// Run through `cargo run --bin generate_lookups`
fn main() -> Result<(), Custom> {
    let config = convenience::load_config("config.toml");
    let id_to_name_str: String = String::deserialize(config["filepaths"]["data"]["id_to_name"].clone())?;
    let name_to_id_str: String = String::deserialize(config["filepaths"]["data"]["name_to_id"].clone())?;
    
    let mapping_path: String = String::deserialize(config["filepaths"]["data"]["mapping"].clone())?;

    dbg!(&mapping_path);

    let logger_path: &str = "src/bin/genlookups.log"; // Something to do with config
    let logger_config = LogConfig::new(logger_path, Level::Debug, Format::Compact);
    let logger = logger_config.create_logger();
    debug!(&logger, "Initialised logger.");
    
    let mut mapping_io = LogFileIO::<&str>::with_options(
        &logger,
        &mapping_path,
        [true, true, true],
    );

    let mapping: Vec<MappingItem> = match mapping_io.read() {
        Ok(c) => c,
        Err(e) => log_panic!(
            &logger,
            Level::Error,
            "Failed to parse mapping file: {}",
            e
        ),
    };

    let mut id_to_name = HashMap::<String, String>::with_capacity(mapping.len());
    let mut name_to_id = HashMap::<String, String>::with_capacity(mapping.len());

    for item in mapping  {
        let item_id = item.id.to_string();
        let item_name = item.name.to_string();

        id_to_name.insert(item_id.clone(), item_name.clone());
        name_to_id.insert(item_name, item_id);
    }

    // Overwrite file contents
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    
    mapping_io.set_file_path(&id_to_name_str);
    if mapping_io.clear_contents().is_err() {
        warn!(
            &logger,
            "Failed to clear contents of id_to_name"
        );
    };
    if let Err(e) = mapping_io.write(&id_to_name, formatter.clone()) {
        log_panic!(
            &logger,
            Level::Error,
            "Failed to write id_to_name: {}",
            e
        );
    }

    mapping_io.set_file_path(&name_to_id_str);
    if mapping_io.clear_contents().is_err() {
        warn!(
            &logger,
            "Failed to clear contents of name_to_id"
        );
    };
    if let Err(e) = mapping_io.write(&name_to_id, formatter.clone()) {
        log_panic!(
            &logger,
            Level::Error,
            "Failed to write name_to_id: {}",
            e
        );
    }


    Ok(())
}