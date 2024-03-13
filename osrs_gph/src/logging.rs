use crate::{
    api::{APIHeaders, API},
    data_types::PriceDataType,
    errors::{Custom, CustomResult},
    file_io::FileIO,
    item_search::{Item, ItemSearch, Recipe, RecipeBook}, price_handle::PriceHandle,
};

use std::{
    any::Any,
    collections::HashMap,
    fmt::{self, Debug, Display},
    fs::File,
    io::{self, BufReader, BufWriter, Seek},
    path::Path,
    time::Instant,
};

use prettytable::{Table, Row, Cell};
use reqwest::{
    blocking::{self, Response},
    header::HeaderMap,
};

use serde::{Deserialize, Serialize};

use slog::{debug, error, info, warn, Level, Logger};

use sloggers::{
    file::FileLoggerBuilder,
    types::{Format, Severity},
    Build,
};

pub type LogFileIO<'l, S> = Logging<'l, FileIO<S>>;
pub type LogAPI<'l, S> = Logging<'l, API<S>>;
pub type LogItemSearch<'l, 'io, S> = Logging<'l, ItemSearch<'io, S>>;
pub type LogRecipeBook<'l> = Logging<'l, RecipeBook>;
pub type LogPriceHandle<'l, 'il, S> = Logging<'l, PriceHandle<'il, S>>;

/// Derived from `info!` from slog.
/// Logs message then panics.
#[macro_export]
macro_rules! log_panic{
    ($l:expr, $le:expr, #$tag:expr, $($args:tt)+) => {
        {slog::log!($l, $le, $tag, $($args)+);
        panic!($($args)+)}
    };
    ($l:expr, $le:expr, $($args:tt)+) => {
        {slog::log!($l, $le, "", $($args)+);
        panic!($($args)+)}
    };
}

#[macro_export]
macro_rules! log_warning {
    ($l:expr, #$tag:expr, $($args:tt)+) => {
        {slog::log!($l, Level::Warning, $tag, $($args)+)}
    };
    ($l:expr, $($args:tt)+) => {
        {slog::log!($l, Level::Warning, "", $($args)+)}
    };
}

pub(crate) use log_panic;
pub(crate) use log_warning;

#[derive(Debug)]
pub struct Logging<'a, T> {
    pub logger: &'a Logger,
    pub object: T,
}

#[derive(Debug)]
pub struct LogConfig<S: AsRef<Path>> {
    pub filename: S,
    pub log_level: Level,
    pub log_style: Format, // Eventually this will be a custom format
}

impl<S: AsRef<Path>> From<S> for LogConfig<S> {
    fn from(value: S) -> Self {
        Self::new(value, Level::Trace, Format::Compact)
    }
}

impl<S: AsRef<Path>> LogConfig<S> {
    pub fn new(filename: S, log_level: Level, log_style: Format) -> Self {
        LogConfig {
            filename,
            log_level,
            log_style,
        }
    }
    fn level_to_severity(&self) -> Severity {
        match &self.log_level {
            Level::Trace => Severity::Trace,
            Level::Debug => Severity::Debug,
            Level::Info => Severity::Info,
            Level::Warning => Severity::Warning,
            Level::Error => Severity::Error,
            Level::Critical => Severity::Critical,
        }
    }
    /// Creates (file) logger from provided config
    /// # Panics
    /// Panics when building Logger fails.
    pub fn create_logger(&self) -> Logger {
        // Logger config
        let mut logger_builder: FileLoggerBuilder = FileLoggerBuilder::new(&self.filename);
        logger_builder.level(self.level_to_severity());
        logger_builder.format(self.log_style); // Eventually will replace with custom
        logger_builder.truncate(); // Want to remove old logs first

        // Create actual logger from config
        match logger_builder.build() {
            Ok(l) => l,
            Err(e) => panic!("{}", e),
        }
    }
}

impl<'l, S: AsRef<Path> + fmt::Display> LogFileIO<'l, S> {
    /// See [`Self::with_options`] for specifying custom options.
    /// options: (Read: true, Write: false, Create: false)
    pub fn new(logger: &'l Logger, filename: S) -> Self {
        Self::with_options(logger, filename, [true, false, false])
    }

    /// Creates File object which can be handled.
    /// options: (Read, Write, Create)
    pub fn with_options<O: Into<Option<[bool; 3]>>>(
        logger: &'l Logger,
        filename: S,
        options: O,
    ) -> LogFileIO<'l, S> {
        // Treating `create` as overwrite existing as well.
        let [read, write, create] = options
            .into()
            .or_else(|| {
                log_panic!(
                    &logger,
                    Level::Critical,
                    "Failed to instantiate file options."
                )
            })
            .unwrap_or([true, false, false]);

        Self {
            logger,
            object: FileIO::new(filename, [read, write, create]),
        }
    }
    // Is this the best way?
    fn file(&mut self) -> File {
        let [read, write, create] = self.object.options;
        let filename = &self.object.filename;
        match std::fs::OpenOptions::new()
            .read(read)
            .write(write)
            .create(create)
            .open(filename)
        {
            Ok(f) => {
                info!(&self.logger, "Opened file {}", &filename);
                f
            }
            Err(e) => log_panic!(&self.logger, Level::Critical, "{}", e),
        }
    }

    /// # Errors
    /// Errors if `Self::rewind` fails.
    pub fn open_file<E: fmt::Display>(&mut self, emsg: E) -> CustomResult<File> {
        let mut file = self.file(); // Open once
        if !self.has_data(&file) {
            warn!(&self.logger, "{emsg}");
            return Err(io::ErrorKind::NotFound.into());
        };
        self.rewind(&mut file);
        Ok(file)
    }

    fn rewind(&self, file: &mut File) {
        // Need to rewind cursor just in case this isn't first operation
        let Ok(curr_pos) = file.stream_position() else {
            log_panic!(
                &self.logger,
                Level::Error,
                "Error seeking cursor of {}",
                &self.object.filename
            )
        };

        if curr_pos == 0 {
            return; // Early exit. Don't rewind if not needed.
        }

        if let Ok(()) = (file).rewind() {
            info!(&self.logger, "Rewound cursor of {}", &self.object.filename);
        } else {
            log_panic!(
                &self.logger,
                Level::Error,
                "Error rewinding cursor of {}",
                &self.object.filename
            );
        }
    }

    pub fn has_data(&self, f: &File) -> bool {
        if let Ok(m) = self.object.metadata(f) {
            m.len() > 0
        } else {
            log_panic!(
                &self.logger,
                Level::Critical,
                "Couldn't read metadata from {}",
                &self.object.filename
            )
        }
    }

    /// # Errors
    /// When file does not exist or serialization fails.
    pub fn write<J: Serialize, F: serde_json::ser::Formatter>(
        &mut self,
        data: &J,
        format: F,
    ) -> Result<(), Custom> {
        let file = self.open_file(
            format!("Attempted to write to non-existent file {}", &self.object.filename)
        )?;
        info!(&self.logger, "Overwriting file {}", &self.object.filename);

        let buffer = BufWriter::with_capacity(self.object.get_buf_size(), file);
        let mut serialiser = serde_json::ser::Serializer::with_formatter(buffer, format);

        // DEBUG
        let now = Instant::now();
        data.serialize(&mut serialiser)?;
        println!("Wrote file in {:?}", now.elapsed()); // DEBUG

        Ok(())
    }

    /// # Errors
    /// Errors when the file does not exist or deserialization fails.
    pub fn read<'de, T: Deserialize<'de>>(&mut self) -> Result<T, Custom> {
        let file = self.open_file(
            format!("Attempted to read non-existent file {}", &self.object.filename)
        )?;
        info!(&self.logger, "Reading file {}", &self.object.filename);

        // TODO: Is file.read_to_string(...); faster?
        let buffer = BufReader::with_capacity(self.object.get_buf_size(), file); // Speedy
        let mut deserialiser = serde_json::de::Deserializer::from_reader(buffer);

        // DEBUG
        let now = Instant::now();
        let t = T::deserialize(&mut deserialiser)?;
        println!("Read file in {:?}", now.elapsed()); // DEBUG
        
        Ok(t)
    }
}

impl<'l, S: AsRef<str> + Display> LogAPI<'l, S> {
    pub fn new(logger: &'l Logger, object: API<S>) -> Self {
        Self { logger, object }
    }
    /// Makes API request and returns the JSON response
    pub fn request<R: Any, E: AsRef<str> + Display, F: FnOnce(Response) -> R>(
        &self,
        endpoint: E,
        callback: F,
        headers: Option<APIHeaders>,
    ) -> R {
        // Merge headers prioritising new
        let header_map: HeaderMap = match self.object.add_headers(headers).try_into() {
            Ok(h) => h,
            Err(e) => log_panic!(&self.logger, Level::Critical, "Header_map error: {}", e),
        };

        let u = self.object.api_url.to_string() + endpoint.as_ref();

        let client = blocking::Client::new();
        let res_build = client.get(u).headers(header_map);

        let now = Instant::now();
        match res_build.send() {
            Ok(b) => {
                info!(
                    &self.logger,
                    "Request sent to {}. Took {:?}",
                    endpoint,
                    now.elapsed()
                );
                debug!(&self.logger, "Performing callback on JSON");
                callback(b)
            }
            Err(e) => log_panic!(&self.logger, Level::Critical, "Request sent error: {}", e),
        }
    }
}

// 'a (Logger) outlives 'b (Object)
impl<'l: 'io, 'io, S: AsRef<Path> + std::fmt::Display> LogItemSearch<'l, 'io, S> {
    pub fn new<H: Into<HashMap<String, Item>>>(
        logger: &'l Logger,
        price_data_handler: LogFileIO<'l, S>,
        id_to_name_handler: LogFileIO<'l, S>,
        name_to_id_handler: LogFileIO<'l, S>,
        items: Option<H>,
    ) -> Self {
        let h = if let Some(il) = items {
            il.into()
        } else {
            HashMap::new()
        };
        Self {
            logger,
            object: ItemSearch::<'io, S>::new(
                price_data_handler,
                id_to_name_handler,
                name_to_id_handler,
                h,
            )
        }
    }

    pub fn initalize(&mut self) {
        debug!(&self.logger, "Initalising ItemSearch");
        self.populate_lookups();
        self.populate_items();
    }

    fn populate_id_to_name(&mut self) {
        let i2n = &mut self.object.id_to_name_handler;
        self.object.id_to_name = match i2n.read::<HashMap<String, String>>() {
            Ok(o) => o,
            Err(e) => log_panic!(
                &self.logger,
                Level::Error,
                "Failed to populate id_to_name: {}",
                e
            ),
        };
    }
    fn populate_name_to_id(&mut self) {
        let n2i = &mut self.object.name_to_id_handler;
        self.object.name_to_id = match n2i.read() {
            Ok(o) => o,
            Err(e) => log_panic!(
                &self.logger,
                Level::Error,
                "Failed to populate id_to_name: {}",
                e
            ),
        }
    }
    pub fn populate_lookups(&mut self) {
        self.populate_name_to_id();
        self.populate_id_to_name();
    }

    pub fn ignore_items(&mut self, ignore: &[String]) -> i32 {
        debug!(&self.logger, "Removing ignored items.");
        match ignore
            .iter()
            .filter_map(|x| self.object.items.remove(x))
            .count()
            .try_into()
        {
            Ok(n) => {
                debug!(&self.logger, "Removed {n} ignored items.");
                n
            }
            Err(e) => log_panic!(
                &self.logger,
                Level::Error,
                "Number of ignored items is too big: {}",
                e
            ),
        }
    }

    pub fn populate_items(&mut self) {
        // Load item data
        let item_data = match self.object.price_data_handler.read::<PriceDataType>() {
            Ok(d) => {
                info!(&self.logger, "Read success.");
                d.data
            }
            Err(e) => log_panic!(&self.logger, Level::Error, "Failed to read file: {:?}", e),
        }; // item_id(String) => item_datum

        // Create item object for each item
        for (item_id, item_datum) in item_data {
            // Check if item_data is even valid
            if item_datum.invalid_data() {
                continue;
            }

            let item_name = if let Some(s) = self.object.id_to_name.get(&item_id) {
                s.to_string()
            } else {
                log_warning!(
                    &self.logger,
                    "Item ID {item_id:?} not found in {}",
                    &self.object.id_to_name_handler.object.filename
                );
                continue;
            };

            // Otherwise create item and append
            let item = Item::new(item_name.clone(), item_id, item_datum);

            self.object.items.insert(item_name, item);
        }
    }

    pub fn item_by_name(&self, item_name: &String) -> Option<&Item> {
        let item = self.object.item_by_name(item_name);
        if item.is_none() {
            warn!(&self.logger, "Item `{item_name}` not found.");
        }
        item
    }  
    pub fn item_by_id(&self, item_id: &String) -> Option<&Item> {
        self.object.item_by_id(item_id)
    }
}

impl<'l> LogRecipeBook<'l> {
    #[must_use]
    pub fn new(logger: &'l Logger, object: RecipeBook) -> Self {
        Self { logger, object }
    }
    pub fn initalize<IS: AsRef<Path>, S: AsRef<Path> + fmt::Display, R: Into<Recipe>>(
        &mut self,
        all_items: &Logging<ItemSearch<IS>>,
        recipe_path: S,
        recipes: Option<Vec<R>>,
    ) {
        if let Some(recipe_list) = recipes {
            // Need to convert each item into Recipe
            let parsed: Vec<Recipe> = recipe_list.into_iter().map(Into::into).collect();
            self.object.add_from_list(parsed);
            self.object.remove_recipe("Template");
        } else {
            self.load_default_recipes(all_items, recipe_path);
        }
    }
    #[must_use]
    pub fn get_recipe(&self, recipe_name: &String) -> Option<&Recipe> {
        let or = self.object.get_recipe(recipe_name);
        if or.is_none() {
            error!(&self.logger, "Invalid recipe name: {}", &recipe_name);
        }
        or
    }
    pub fn load_default_recipes<IS: AsRef<Path>, S: AsRef<Path> + fmt::Display>(
        &mut self,
        _all_items: &Logging<ItemSearch<IS>>,
        recipe_path: S,
    ) {
        let mut recipes_fio = Logging::<FileIO<S>>::new(self.logger, recipe_path);
        let mut recipe_list: Vec<Recipe> = match recipes_fio.read::<HashMap<String, Recipe>>() {
            Ok(l) => l.into_values().collect(),
            Err(e) => log_panic!(&self.logger, Level::Error, "Failed to load recipes. {}", e),
        };

        // Filer out invalid recipes; using .isvalid()
        // Log any invalid recipes
        let before_len = recipe_list.len();
        recipe_list.retain(|r| {
            if r.isvalid() {
                true
            } else {
                log_warning!(&self.logger, "Skipping recipe: {}", r.name);
                false
            }
        });
        debug!(
            &self.logger,
            "Filtered out {} invalid recipes.",
            before_len - recipe_list.len()
        );

        self.object.add_from_list(recipe_list);
        self.object.remove_recipe("Template");
        debug!(&self.logger, "Loaded {} recipes.", self.object.len());
    }
}


impl<'l: 'il, 'il, S: AsRef<Path> + Display> LogPriceHandle<'l, 'il, S> {
    pub fn new(logger: &'l Logger, object: PriceHandle<'il, S>) -> Self {
        Self { logger, object }
    }

    pub fn recipe_price_overview(&self, recipe_name: &String) -> Option<Row> {
        let recipe = self.object.recipe_list.get_recipe(recipe_name)?;
        self.recipe_price_overview_from_recipe(recipe)
    }
    pub fn recipe_price_overview_from_recipe(&self, recipe: &Recipe) -> Option<Row> {
        // Need to parse item strings into Item objects
        let input_items = self.object.parse_item_list(&recipe.inputs)?;
        let output_items = self.object.parse_item_list(&recipe.outputs)?;

        let input_details = PriceHandle::<S>::item_list_prices_unchecked(
            input_items, true
        );
        let output_details = PriceHandle::<S>::item_list_prices_unchecked(
            output_items, false
        ); 

        let cost = PriceHandle::<S>::total_price(
            &input_details.into_values().collect::<Vec<_>>()
        );
        let revenue = PriceHandle::<S>::apply_tax(
            PriceHandle::<S>::total_price(
                &output_details.into_values().collect::<Vec<_>>()
            )
        );
        let profit = revenue-cost;
        let time = &recipe.time;

        Some(Row::new(
            vec![
                Cell::new(cost.to_string().as_str()), 
                Cell::new(revenue.to_string().as_str()), 
                Cell::new(profit.to_string().as_str()), 
                Cell::new(time.to_string().as_str())
            ]
        ))
    }
}