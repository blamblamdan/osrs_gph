use crate::{
    api::{APIHeaders, FromTable, API},
    convenience::{self, Input, RIGHT_ALIGN, LEFT_ALIGN},
    data_types::PriceDataType,
    errors::Custom,
    item_search::{Item, Recipe, RecipeBook},
    log_panic,
    logging::{LogAPI, LogConfig, LogFileIO, LogItemSearch, LogRecipeBook, LogPriceHandle},
    pareto_sort::compute_weights, price_handle::PriceHandle,
};
use prettytable::{Row, Cell};
use crate::convenience::FORMAT_MARKDOWN;
use core::fmt;
use std::{
    collections::HashMap,
    io::{BufReader, Read},
    path::Path,
};

use reqwest::blocking::Response;
use serde::Deserialize;
use slog::{debug, info, Level, Logger, error, warn};
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
    let (logger, results_fps, optimal_overview) = main_inner();
    write_results(&logger, &results_fps, &optimal_overview);
}

#[must_use]
/// `(logger, results_fps, optimal_overview)`
pub fn main_inner() -> (Logger, Table, prettytable::Table){
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
    let results_fps: &Table = config["filepaths"]["results"].as_table().unwrap_or_else(|| {
        log_panic!(
            &logger,
            Level::Critical,
            "Results filepaths could not be parsed"
        )
    });

    let (mut price_data_io, name_to_id, id_to_name) = create_fio(&logger, data_fps);

    // When Logging<ItemSearch> is initialised => Simply load data **from file** to populate object
    let inp = price_data_io
        .logger
        .input("1. API Refresh Data\n2. Load previous Data\n");
    let choice = inp.trim_end();
    // Load new data from API or pre-existing file data


    refresh_prices(&logger, choice, &config, &mut price_data_io);

    let (mut item_search_s, ignore_items) =
        create_item_search(&logger, &mut price_data_io, &id_to_name, &name_to_id, &config);

    // Setup ItemSearch
    item_search_s.initalize();
    item_search_s.ignore_items(&ignore_items);

    let (recipe_fp, mut recipe_book) = create_recipe_book(&logger, &config);
    recipe_book.initalize(&item_search_s, &recipe_fp, None::<Vec<Recipe>>);

    let (price_hand, backend_settings, weights) = create_price_handle(&logger, &config, item_search_s, recipe_book);
    // dbg!(&weights);
    let mut optimal_overview = price_hand.all_recipe_overview(&weights, backend_settings);

    //
    optimal_overview.set_format(*FORMAT_MARKDOWN);
    optimal_overview.set_titles(
        Row::new(
            vec![
            Cell::new_align("Method", LEFT_ALIGN),
            Cell::new_align("Loss/Gain", RIGHT_ALIGN),
            Cell::new_align("Total Loss/Gain", RIGHT_ALIGN),
            Cell::new_align("Time (h)", RIGHT_ALIGN),
            Cell::new_align("GP/h", RIGHT_ALIGN)
            ]
        )
    );

    (logger, results_fps.clone(), optimal_overview)

}

pub fn refresh_prices<'l>(logger: &'l Logger, choice: &str, config: &Table, price_data_io: &mut LogFileIO<'l, &'l str>) {
    // This is independent of fileIO creation
    // Only matters when the *data* should be updated
    if choice == "1" {
        info!(&logger, "Retrieving prices from API.");
        perform_api_operations(config, logger, price_data_io);
    } else if choice == "2" {
        info!(&logger, "Loading previous data instead.");
    } else {
        log_panic!(&logger, Level::Error, "Bad choice {}", &choice);
    }
}

fn write_results(logger: &Logger, results_fps: &toml::map::Map<String, Value>, optimal_overview: &prettytable::Table) {
    let mut result_writer_fio = LogFileIO::<&str>::with_options(
        logger,
        results_fps["optimal"]
            .as_str()
            .unwrap_or("results/optimal_overview.txt"),
        [true, true, true]
    );
    if let Err(e) = result_writer_fio.clear_contents() {
        warn!(&logger, "Failed to clear file contents. {}", e);
    }

    match optimal_overview.print(&mut result_writer_fio.get_writer().ok().unwrap()) {
        Ok(_) => info!(&logger, "Sucessfully wrote results."),
        Err(e) => error!(&logger, "Failed to write results: {}", e)
    }
}

// pub fn write_results() {

// }
#[must_use]
pub fn create_price_handle<'l: 'io, 'io>(logger: &'l Logger, config: &Table, item_search_s: LogItemSearch<'l, 'io, &'io str>, recipe_book: LogRecipeBook<'l>) -> (LogPriceHandle<'l, 'io, &'io str>, [bool; 3], [f32; 4]) {
    // TODO compute weights, price_calc and display
    let coins = match i32::deserialize(config["profit_settings"]["money"]["coins"].clone()) {
        Ok(c) => c,
        Err(e) => log_panic!(
            &logger,
            Level::Error,
            "Failed to parse number of coins: {}",
            e
        ),
    };
    let pmargin =
        match f32::deserialize(config["profit_settings"]["money"]["percent_margin"].clone()) {
            Ok(c) => c,
            Err(e) => log_panic!(
                &logger,
                Level::Error,
                "Failed to parse percent margin: {}",
                e
            ),
        };
    let weights: [f32; 4] =
        match HashMap::<String, f32>::deserialize(config["profit_settings"]["weights"].clone()) {
            Ok(w) => {
                let v = [w["margin_to_time"], w["time"], w["gp_h"]];
                compute_weights(coins, v)
            }
            Err(e) => log_panic!(&logger, Level::Error, "Failed to parse weights: {}", e),
        };

    // let row = price_hand.recipe_price_overview(&"Humidify Clay".to_string());
    // dbg!(&row);
    let backend_settings: [bool; 3] = match HashMap::<String, bool>::deserialize(
        config["profit_settings"]["display"]["backend"].clone()
    ) {
        Ok(s) => [s["profiting"],s["show_hidden"],s["reverse"]],
        Err(e) => log_panic!(&logger, Level::Error, "Failed to parse back-end display settings: {}", e),
    };


    let price_hand = LogPriceHandle::new(logger,
        PriceHandle::new(item_search_s, recipe_book, coins, pmargin)
    );
    (price_hand, backend_settings, weights)
}

pub fn perform_api_operations(config: &Table, logger: &Logger, price_data_io: &mut LogFileIO<&str>) {
    // Setup the API stuff
    let api = if let Some(api_settings) = config["API_settings"].as_table() {
        info!(
            &logger,
            "Initialising: API settings for {}", &api_settings["url"]
        );

        setup_api(logger, api_settings)
    } else {
        log_panic!(&logger, Level::Critical, "API settings could not be parsed")
    };

    let api_data = api_request(&api);

    if let Err(e) = price_data_io.clear_contents() {
        warn!(&price_data_io.logger, "Failed to clear file contents. {}", e);
    }
    if let Err(e) = write_api_data(price_data_io, &api_data) {
        log_panic!(
            &price_data_io.logger,
            Level::Error,
            "Failed to write to file: {:?}",
            e
        );
    } else {
        info!(&price_data_io.logger, "Write success.");
    }
}

#[must_use]
pub fn create_recipe_book<'l>(logger: &'l Logger, config: &toml::map::Map<String, Value>) -> (String, LogRecipeBook<'l>) {
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

#[must_use]
pub fn create_item_search<'l: 'io, 'io: 'l + 'fp, 'fp>(
    logger: &'l Logger,
    price_data_io: &'io mut LogFileIO<'io, &'fp str>,
    id_to_name: &'io LogFileIO<'io, &'fp str>,
    name_to_id: &'io LogFileIO<'io, &'fp str>,
    config: &Table,
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

#[must_use]
pub fn create_fio<'l, 'd>(
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

#[must_use]
pub fn api_request(log_api: &LogAPI<String>) -> PriceDataType {
    let callback = |mut r: Response| -> Result<PriceDataType, Custom> {
        let buffer = BufReader::new(r.by_ref()); // 400KB (So far the responses are 395KB 2024-02-02)
        Ok(serde_json::de::from_reader::<_, PriceDataType>(buffer)?)
    };
    match log_api.request("/latest".to_string(), callback, None) {
        Ok(d) => {
            debug!(&log_api.logger, "Deserialised API response.");
            d
        }
        Err(e) => log_panic!(&log_api.logger, Level::Critical, "{}", e),
    }
}

pub fn write_api_data<S: AsRef<Path> + fmt::Display>(
    price_data_io: &mut LogFileIO<S>,
    api_data: &PriceDataType,
) -> Result<(), crate::errors::Custom> {
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    price_data_io.write(&api_data, formatter.clone())
}

#[must_use]
pub fn setup_api_headers(logger: &Logger, headers: &Value) -> APIHeaders {
    if let Some(a) = headers.as_table() {
        APIHeaders::from_table_ref(a)
    } else {
        log_panic!(logger, Level::Critical, "Auth headers could not be parsed")
    }
}

#[must_use]
pub fn setup_api<'a>(logger: &'a Logger, api_settings: &Table) -> LogAPI<'a, String> {
    // API Headers from config
    let headers = setup_api_headers(logger, &api_settings["auth_headers"]);

    if let Ok(api_url) = String::deserialize(api_settings["url"].clone()) {
        LogAPI::new(logger, API::new(api_url, headers))
    } else {
        log_panic!(logger, Level::Critical, "API url could not be parsed")
    }
}