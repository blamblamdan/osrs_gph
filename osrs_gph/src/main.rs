#![allow(unused_imports)]
use core::fmt;
use std::any::Any;

use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::fs::{File, Metadata};
use std::hash::Hash;
use std::io::{BufReader, BufWriter, Read};

use std::path::{Path, PathBuf};
use std::str::FromStr;

use osrs_gph::errors::CustomErrors;
use osrs_gph::item_search::{Item, ItemSearch, Recipe, RecipeBook};
use reqwest::blocking::Response;
use reqwest::header::{HeaderMap, WARNING};
use serde::{Deserialize, Serialize};

use slog::{crit, debug, error, info, log, trace, warn, Level, Logger};
use sloggers::types::{self, Format, Severity};
use toml::{Table, Value};

use osrs_gph::convenience::Input;
use osrs_gph::convenience::{self, input};
use osrs_gph::logging::{LogConfig, Logging};

use osrs_gph::log_panic;

use osrs_gph::api::{APIHeaders, FromTable, API};
use osrs_gph::file_io::FileIO;

use osrs_gph::data_types::PriceDataType;

// use url::{Url, ParseError, ParseOptions};
use reqwest::{blocking, IntoUrl};

use serde::de::{DeserializeOwned, IntoDeserializer};

#[allow(unused_macros)]
macro_rules! early_exit {
    () => {
        panic!("Exiting early...");
    };
}

fn main() {
    let config = convenience::load_config("config.toml"); // Load TOML file into here
    let logger_path: &str = config["filepaths"]["logging"]["log_file"]
        .as_str()
        .unwrap_or("runtime.log"); // Something to do with config
                                   // Logger config
    let logger_config = LogConfig::new(logger_path, Level::Debug, Format::Compact);

    let logger = logger_config.create_logger();
    debug!(&logger, "Initialised logger.");

    // // Load all items into memory
    let data_fps: &Table = &config["filepaths"]["data"].as_table().unwrap_or_else(|| {
        log_panic!(
            &logger,
            Level::Critical,
            "Data filepaths could not be parsed"
        )
    });

    // let price_data_io = Logging<FileIO>::new(data_fps["price_data"].to_string());
    let mut price_data_io = Logging::<FileIO<&str>>::with_options(
        &logger,
        data_fps["price_data"]
            .as_str()
            .unwrap_or("api_data/price_data.json"),
        [true, true, true],
    );
    // price_data_io.set_buf_size(8192usize); // DEBUG

    let name_to_id = Logging::<FileIO<&str>>::new(
        &logger,
        data_fps["name_to_id"]
            .as_str()
            .unwrap_or("lookup_data/name_to_id.json"),
    );

    let id_to_name = Logging::<FileIO<&str>>::new(
        &logger,
        data_fps["id_to_name"]
            .as_str()
            .unwrap_or("lookup_data/id_to_name.json"),
    );

    info!(&logger, "Initalised all FileIO structs");

    // `price_data_io` FileIO object is consumed by Logging
    // **FUTURE** `price_data_io` is entirely consumed by Logging<PriceAPI> object

    // When Logging<ItemSearch> is initialised => Simply load data **from file** to populate object
    let choice = price_data_io
        .logger
        .input("1. API Refresh Data\n2. Load previous Data\n");

    // Load new data from API or pre-existing file data
    match (&choice).trim_end() {
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

    let mut item_search_s = Logging::<ItemSearch<&str>>::new::<HashMap<String, Item>>(
        &logger,
        price_data_io,
        id_to_name,
        name_to_id,
        None,
    );
    // TODO: Need to convert PriceDataType (String => PriceDatum) to String=>Item (Using name_to_id)

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

    // Setup ItemSearch
    item_search_s.initalize();
    item_search_s.ignore_items(ignore_items);

    // Load recipes
    let recipe_fp:String = match String::deserialize(config["filepaths"]["recipes"]["recipe_data"].clone()) {
        Ok(fp) => fp,
        Err(e) => log_panic!(
            &logger,
            Level::Error,
            "Failed to parse recipe filepath: {}",
            e
        ),
    };
    info!(&logger, "Loading: Recipes from {}", &recipe_fp);
    let mut recipe_book = Logging::<RecipeBook>::new(
        &logger,
        RecipeBook::default()
    );
    recipe_book.initalize(&item_search_s, &recipe_fp, None::<Vec<Recipe>>);

    // TODO compute weights, price_calc and display
    let coins = match i32::deserialize(config["profit_settings"]["money"]["coins"].clone()) {
        Ok(c) => c,
        Err(e) => log_panic!(
            &logger,
            Level::Error,
            "Failed to parse number of coins: {}",
            e
        )
    };
    let pmargin = match f32::deserialize(config["profit_settings"]["money"]["percent_margin"].clone()) {
        Ok(c) => c,
        Err(e) => log_panic!(
            &logger,
            Level::Error,
            "Failed to parse percent margin: {}",
            e
        )
    };
    let weights = match HashMap::<String, f32>::deserialize(config["profit_settings"]["weights"].clone()) {
        Ok(w) => w,
        Err(e) => log_panic!(
            &logger,
            Level::Error,
            "Failed to parse weights: {}",
            e
        )
    };
}

fn api_request(log_api: &Logging<'_, API<String>>) -> PriceDataType {
    let callback = |mut r: Response| -> Result<PriceDataType, CustomErrors> {
        let buffer = BufReader::new(r.by_ref()); // 400KB (So far the responses are 395KB 2024-02-02)
        match serde_json::de::from_reader(buffer) {
            Ok(o) => Ok(o),
            Err(e) => Err(e.into()),
        }
    };
    match log_api.request("/latest".to_string(), &callback, None) {
        Ok(d) => {
            debug!(&log_api.logger, "Deserialised API response.");
            d
        }
        Err(e) => log_panic!(&log_api.logger, Level::Critical, "{}", e),
    }
}

fn write_api_data<S: AsRef<Path> + fmt::Display>(
    price_data_io: &mut Logging<'_, FileIO<S>>,
    api_data: &PriceDataType,
) -> Result<(), ()> {
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    match price_data_io.write(&api_data, formatter.clone()) {
        Ok(()) => Ok(()),
        Err(e) => log_panic!(price_data_io.logger, Level::Critical, "{}", e),
    }
}

fn setup_api_headers(logger: &Logger, headers: &Value) -> APIHeaders {
    match headers.as_table() {
        Some(a) => APIHeaders::from_table_ref(a),
        None => log_panic!(logger, Level::Critical, "Auth headers could not be parsed"),
    }
}

fn setup_api<'a>(logger: &'a Logger, api_settings: &Table) -> Logging<'a, API<String>> {
    // API Headers from config
    let headers = setup_api_headers(logger, &api_settings["auth_headers"]);

    let api_url = String::deserialize(api_settings["url"].clone())
        .unwrap_or_else(|_| log_panic!(logger, Level::Critical, "API url could not be parsed"));

    Logging::<API<String>>::new(logger, API::new(api_url, headers))
}
