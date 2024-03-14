use crate::log_panic;

use std::{
    fmt::Display,
    io::{prelude::BufRead, Write},
    path::Path,
};

use prettytable::{Row, format::{FormatBuilder, LinePosition, TableFormat, LineSeparator}};
use slog::{debug, Level, Logger};
use toml::Table;
use lazy_static::lazy_static;
pub static  CENTER_ALIGN: prettytable::format::Alignment = prettytable::format::Alignment::CENTER;
pub static LEFT_ALIGN: prettytable::format::Alignment = prettytable::format::Alignment::LEFT;
pub static RIGHT_ALIGN: prettytable::format::Alignment = prettytable::format::Alignment::RIGHT;

lazy_static!{
    pub static ref CUSTOM_TABLE_FORMAT: TableFormat = prettytable::format::FormatBuilder::new()
        .column_separator(' ')
        // .borders('|')
        .separators(
            &[
                // format::LinePosition::Top, 
                prettytable::format::LinePosition::Bottom
            ],
            prettytable::format::LineSeparator::new('#', '#', '#', '#'),
        )
        .separator(prettytable::format::LinePosition::Title, prettytable::format::LineSeparator::new('-', ' ', ' ', ' '))
        .padding(1, 1)
        .build();
    pub static ref FORMAT_MARKDOWN: TableFormat = FormatBuilder::new()
        .padding(1, 1)
        .borders('|')
        .separator(LinePosition::Title, LineSeparator::new('-', '|', '|', '|'))
        .column_separator('|')
        .build();
}

#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn floor(x: f64) -> i32 {
    x.floor() as i32
}

#[must_use]
/// Digits after decimal point
/// # Panics
/// Panics if trucated float fails to be parsed
pub fn f_round(x: f32, digits: usize) -> f32 {
    format!("{x:.digits$}").parse().unwrap()
}

fn flush_stout() {
    std::io::stdout().flush().ok();
}

/// Loads config.toml file
/// # Panics
/// Panics if config file read fails .
pub fn load_config<P: AsRef<Path>>(fp: P) -> Table {
    // Load file into a String
    let file = match std::fs::read_to_string(&fp) {
        Ok(f) => f,
        Err(e) => panic!("{}", e),
    };

    // Parse into TOML
    match file.parse() {
        Ok(s) => s,
        Err(e) => panic!("{}", e),
    }
}

pub trait Input {
    /// User input convenience function
    fn input<S: AsRef<str> + Display>(&self, message: S) -> String {
        println!("{message}");
        print!(">"); // Caret for user input
        flush_stout(); // To make sure

        let stdin = std::io::stdin();
        let mut buffer: String = String::new();
        match &stdin.lock().read_line(&mut buffer) {
            Ok(_) => buffer,
            Err(e) => panic!("{}", e),
        }
    }
    fn str_input(&self) -> String {
        String::new()
    }
}

/// User input convenience function
/// # Panics
/// Panics if file read fails on a particular line.
pub fn input<S: AsRef<str> + Display>(message: S) -> String {
    println!("{message}");
    print!(">"); // Caret for user input
    flush_stout(); // To make sure

    let stdin = std::io::stdin();
    let mut buffer: String = String::new();
    match &stdin.lock().read_line(&mut buffer) {
        Ok(_) => buffer,
        Err(e) => panic!("{}", e),
    }
}

impl Input for String {
    fn str_input(&self) -> String {
        Input::input(self, self)
    }
}

impl Input for Logger {
    fn input<S: AsRef<str> + Display>(&self, message: S) -> String {
        println!("{message}");
        print!(">"); // Caret for user input
        flush_stout(); // To make sure

        let stdin = std::io::stdin();
        let mut buffer: String = String::new();
        match &stdin.lock().read_line(&mut buffer) {
            Ok(_) => {
                debug!(&self, "Read user input.");
                buffer
            }
            Err(e) => log_panic!(&self, Level::Error, "{}", e),
        }
    }
}

#[must_use]
/// # Panics
/// If unwrap fails.
pub fn parse_overview(overview: &Row) -> [f32;3] {
    let v = overview.iter().map(|c| c.get_content().parse().unwrap())
        .collect::<Vec<f32>>();
    // recipe_cost, margin, time
    [v[0], v[2], v[3]]
}



pub fn comma_string<T: num_format::ToFormattedStr>(x: &T) -> String {
    // Create a stack-allocated buffer...
    let mut buf = num_format::Buffer::default();

    // Write "1,000,000" into the buffer...
    buf.write_formatted(x, &num_format::Locale::en);

    // Get a view into the buffer as a &str...
    buf.to_string()
}

pub fn parse_comma_string(x: &str) -> Result<i32, std::num::ParseIntError> {
    x.replace(',', "").parse()
}