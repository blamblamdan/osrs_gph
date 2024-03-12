use osrs_gph::{
    api::{APIHeaders, FromTable, API},
    convenience::{self, Input},
    data_types::PriceDataType,
    errors::Custom,
    item_search::{Item, Recipe, RecipeBook},
    log_panic,
    logging::{LogAPI, LogConfig, LogFileIO, LogItemSearch, LogRecipeBook, Logging},
    pareto_sort::compute_weights,
};

use core::fmt;
use std::{
    collections::HashMap,
    io::{BufReader, Read},
    path::Path,
};

use reqwest::blocking::Response;
use serde::Deserialize;
use slog::{debug, info, Level, Logger};
use sloggers::types::Format;
use toml::{Table, Value};

#[allow(unused_macros)]
macro_rules! early_exit {
    () => {
        panic!("Exiting early...");
    };
}

// #[allow(clippy::too_many_lines)]
fn main() {
    let config = convenience::load_config("config.toml"); // Load TOML file into here

    let logger_path: &str = config["filepaths"]["logging"]["log_file"]
        .as_str()
        .unwrap_or("runtime.log"); // Something to do with config
    let logger_config = LogConfig::new(logger_path, Level::Debug, Format::Compact);
    let logger = logger_config.create_logger();
    debug!(&logger, "Initialised logger.");

    // Load all items into memory
    let data_fps: &Table = config["filepaths"]["data"].as_table().unwrap_or_else(|| {
        log_panic!(
            &logger,
            Level::Critical,
            "Data filepaths could not be parsed"
        )
    });

    let (mut price_data_io, name_to_id, id_to_name) = create_fio(&logger, data_fps);

    // When Logging<ItemSearch> is initialised => Simply load data **from file** to populate object
    let choice = price_data_io
        .logger
        .input("1. API Refresh Data\n2. Load previous Data\n");

    // Load new data from API or pre-existing file data
    match choice.trim_end() {
        "1" => {
            info!(&logger, "Retrieving prices from API.");
            // Setup the API stuff
            let api_settings = config["API_settings"].as_table().unwrap_or_else(|| {
                log_panic!(&logger, Level::Critical, "API settings could not be parsed")
            });
            info!(
                &logger,
                "Initialising: API settings for {}", &api_settings["url"]
            );

            let api = setup_api(&logger, api_settings);

            let api_data = api_request(&api);

            match write_api_data(&mut price_data_io, &api_data) {
                Ok(()) => info!(&price_data_io.logger, "Write success."),
                Err(e) => log_panic!(
                    &price_data_io.logger,
                    Level::Error,
                    "Failed to write to file: {:?}",
                    e
                ),
            };
        }
        "2" => info!(&logger, "Loading previous data instead."),
        _ => log_panic!(&logger, Level::Error, "Bad choice {}", &choice),
    };

    let (mut item_search_s, ignore_items) =
        create_item_search(&logger, price_data_io, id_to_name, name_to_id, &config);

    // Setup ItemSearch
    item_search_s.initalize();
    item_search_s.ignore_items(&ignore_items);

    let (recipe_fp, mut recipe_book) = create_recipe_book(&logger, &config);
    recipe_book.initalize(&item_search_s, &recipe_fp, None::<Vec<Recipe>>);

    // TODO compute weights, price_calc and display
    let _coins = match i32::deserialize(config["profit_settings"]["money"]["coins"].clone()) {
        Ok(c) => c,
        Err(e) => log_panic!(
            &logger,
            Level::Error,
            "Failed to parse number of coins: {}",
            e
        ),
    };
    let _pmargin =
        match f32::deserialize(config["profit_settings"]["money"]["percent_margin"].clone()) {
            Ok(c) => c,
            Err(e) => log_panic!(
                &logger,
                Level::Error,
                "Failed to parse percent margin: {}",
                e
            ),
        };
    let weights: Vec<f32> =
        match HashMap::<String, f32>::deserialize(config["profit_settings"]["weights"].clone()) {
            Ok(w) => {
                let v = vec![w["margin_to_time"], w["time"], w["gp_h"]];
                compute_weights(&v)
            }
            Err(e) => log_panic!(&logger, Level::Error, "Failed to parse weights: {}", e),
        };
    dbg!(&weights);
}

fn create_recipe_book<'l>(logger: &'l Logger, config: &toml::map::Map<String, Value>) -> (String, LogRecipeBook<'l>) {
    // Load recipes
    let recipe_fp: String =
    if let Ok(fp) = String::deserialize(config["filepaths"]["recipes"]["recipe_data"].clone()) {
        fp 
    } else { log_panic!(
        &logger,
        Level::Error,
        "Failed to parse recipe filepath"
    ) };
    info!(&logger, "Loading: Recipes from {}", &recipe_fp);

    let recipe_book = LogRecipeBook::new(logger, RecipeBook::default());
    (recipe_fp, recipe_book)
}

fn create_item_search<'l: 'io, 'io: 'l + 'fp, 'fp>(
    logger: &'l Logger,
    price_data_io: LogFileIO<'io, &'fp str>,
    id_to_name: LogFileIO<'io, &'fp str>,
    name_to_id: LogFileIO<'io, &'fp str>,
    config: &toml::map::Map<String, Value>,
) -> (LogItemSearch<'l, 'io, &'fp str>, Vec<String>) {
    let item_search_s = LogItemSearch::<&str>::new::<HashMap<String, Item>>(
        logger,
        price_data_io,
        id_to_name,
        name_to_id,
        None,
    );

    let ignore_items: Vec<String> =
        match Vec::deserialize(config["filepaths"]["recipes"]["ignore_items"].clone()) {
            Ok(v) => v,
            Err(e) => log_panic!(
                &logger,
                Level::Error,
                "Failed to parse list of ignored items: {}",
                e
            ),
        };
    (item_search_s, ignore_items)
}

fn create_fio<'l, 'd>(
    logger: &'l Logger,
    data_fps: &'d toml::map::Map<String, Value>,
) -> (
    LogFileIO<'l, &'d str>,
    LogFileIO<'l, &'d str>,
    LogFileIO<'l, &'d str>,
) {
    let price_data_io = LogFileIO::<&str>::with_options(
        logger,
        data_fps["price_data"]
            .as_str()
            .unwrap_or("api_data/price_data.json"),
        [true, true, true],
    );

    let name_to_id = LogFileIO::<&str>::new(
        logger,
        data_fps["name_to_id"]
            .as_str()
            .unwrap_or("lookup_data/name_to_id.json"),
    );

    let id_to_name = LogFileIO::<&str>::new(
        logger,
        data_fps["id_to_name"]
            .as_str()
            .unwrap_or("lookup_data/id_to_name.json"),
    );

    info!(&logger, "Initalised all FileIO structs");
    (price_data_io, name_to_id, id_to_name)
}

fn api_request(log_api: &LogAPI<String>) -> PriceDataType {
    let callback = |mut r: Response| -> Result<PriceDataType, Custom> {
        let buffer = BufReader::new(r.by_ref()); // 400KB (So far the responses are 395KB 2024-02-02)
        match serde_json::de::from_reader(buffer) {
            Ok(o) => Ok(o),
            Err(e) => Err(e.into()),
        }
    };
    match log_api.request("/latest".to_string(), callback, None) {
        Ok(d) => {
            debug!(&log_api.logger, "Deserialised API response.");
            d
        }
        Err(e) => log_panic!(&log_api.logger, Level::Critical, "{}", e),
    }
}

fn write_api_data<S: AsRef<Path> + fmt::Display>(
    price_data_io: &mut LogFileIO<S>,
    api_data: &PriceDataType,
) -> Result<(), osrs_gph::errors::Custom> {
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    price_data_io.write(&api_data, formatter.clone())
}

fn setup_api_headers(logger: &Logger, headers: &Value) -> APIHeaders {
    if let Some(a) = headers.as_table() {
        APIHeaders::from_table_ref(a)
    } else {
        log_panic!(logger, Level::Critical, "Auth headers could not be parsed")
    }
}

fn setup_api<'a>(logger: &'a Logger, api_settings: &Table) -> LogAPI<'a, String> {
    // API Headers from config
    let headers = setup_api_headers(logger, &api_settings["auth_headers"]);

    let api_url = String::deserialize(api_settings["url"].clone())
        .unwrap_or_else(|_| log_panic!(logger, Level::Critical, "API url could not be parsed"));

    Logging::<API<String>>::new(logger, API::new(api_url, headers))
}
