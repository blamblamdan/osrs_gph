use std::fmt::Display;
use std::io::Write;
use std::path::Path;
use toml::Table;

use std::io::prelude::BufRead;
use slog::{debug, Level, Logger};


use crate::log_panic;

pub fn floor(x: f32) -> i32 {
    x.floor() as i32
}

/// Digits after decimal point
pub fn f_round(x: f32, digits: usize) -> f32 {
    format!("{:.1$}", x, digits).parse().unwrap()
}

fn flush_stout() {
    std::io::stdout().flush().ok();
}

/// Loads config.toml file
pub fn load_config<P: AsRef<Path>>(fp: P) -> Table{
    // Load file into a String
    let file = match std::fs::read_to_string(&fp) {
        Ok(f) => f,
        Err(e) => panic!("{}",e)
    };
    
    // Parse into TOML
    match file.parse() {
        Ok(s) => s,
        Err(e) => panic!("{}",e)
    }
}



pub trait Input {
    /// User input convenience function
    fn input<S: AsRef<str> + Display>(&self, message: S) -> String{
        println!("{}",message);
        print!(">"); // Caret for user input
        flush_stout(); // To make sure


        let stdin = std::io::stdin();
        let mut buffer: String = String::new();
        match &stdin.lock().read_line(&mut buffer) {
            Ok(_) => buffer,
            Err(e) => panic!("{}",e),
        }
    }
    fn str_input(&self) -> String {String::new()}
}


/// User input convenience function
pub fn input<S: AsRef<str> + Display>(message: S) -> String{
    println!("{}",message);
    print!(">"); // Caret for user input
    flush_stout(); // To make sure


    let stdin = std::io::stdin();
    let mut buffer: String = String::new();
    match &stdin.lock().read_line(&mut buffer) {
        Ok(_) => buffer,
        Err(e) => panic!("{}",e),
    }
}

impl Input for String {
    fn str_input(&self) -> String {
        Input::input(self, self)
    }
}


impl Input for Logger {
    fn input<S: AsRef<str> + Display>(&self, message: S) -> String {
        println!("{}",message);
        print!(">"); // Caret for user input
        flush_stout(); // To make sure


        let stdin = std::io::stdin();
        let mut buffer: String = String::new();
        match &stdin.lock().read_line(&mut buffer) {
            Ok(_) => {debug!(&self, "Read user input.");buffer},
            Err(e) => log_panic!(&self, Level::Error, "{}",e),
        }   
    }
}
