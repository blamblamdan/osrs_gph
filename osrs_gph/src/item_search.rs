use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use super::file_io::FileIO;
use super::logging::Logging;
use serde::de::Visitor;
use serde::Deserialize;
use slog::error;
use super::data_types::PriceDatum;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Item {
    pub name: String, // TODO: Consider switching to &str if not needed.
    pub item_id: String, // i32
    pub item_prices: PriceDatum,
}


pub struct ItemSearch<'a,'b,'c, S: AsRef<Path>> { // Curse of logging wrapper...
    pub price_data_handler: Logging<'a, FileIO<S>>, 
    pub id_to_name_handler: Logging<'b, FileIO<S>>,
    pub name_to_id_handler: Logging<'c, FileIO<S>>,
    pub items: HashMap<String, Item>,
    pub name_to_id: HashMap<String, String>,
    pub id_to_name: HashMap<String, String>
}

#[derive(Debug)]
// #[serde(untagged)]
pub enum RecipeTime {
    Time(f32),
    INVALID
}


impl<'de> Deserialize<'de> for RecipeTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        struct RecipeTimeVisitor;
        impl <'de> Visitor<'de> for RecipeTimeVisitor {
            type Value = RecipeTime;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("f32 or no field")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
                where
                    E: serde::de::Error, {
                Ok(RecipeTime::INVALID)
            }
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error, {
                Ok((v as f32).into())
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error, {
                Ok((v as f32).into())
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error, {
                Ok((v as f32).into())
            }
        }
        let de = deserializer.deserialize_f32(RecipeTimeVisitor);
        if let Err(e) = de {
            if e.to_string().contains("missing field") {
                Ok(RecipeTime::INVALID)
            } else { Err(e) }
        } else {
            de
        }
    }
}



#[derive(Debug, Deserialize, Default)]
pub struct Recipe {
    pub name: String,
    pub inputs: HashMap<String, f32>,
    pub outputs: HashMap<String, f32>,
    pub time: RecipeTime
}

#[derive(Debug, Deserialize, Default)]
pub struct RecipeBook {
    pub recipes: HashMap<String, Recipe>
}

impl<'a,'b,'c, S: AsRef<Path> + std::fmt::Display> ItemSearch<'a,'b,'c, S> {
    pub fn new(price_data_handler: Logging<'a, FileIO<S>>, 
    id_to_name_handler: Logging<'b, FileIO<S>>,
    name_to_id_handler: Logging<'c, FileIO<S>>,
    items: HashMap<String, Item>) -> Self { // Using Item Name(String)=>Item(Object)
        Self { price_data_handler, id_to_name_handler, name_to_id_handler, items,
        name_to_id: HashMap::new(), id_to_name: HashMap::new()}
    }
}

impl Item{
    pub fn new(name: String, id: String, price_data: PriceDatum) -> Self {
        Self{name, item_id: id, item_prices: price_data}
    }
    pub fn invalid_data(&self) -> bool {
        self.item_prices.invalid_data()
    }
}

impl Recipe {
    pub fn new<S: Into<String>, T: Into<RecipeTime>>(name: S, inputs: HashMap<String, f32>, outputs: HashMap<String, f32>, time: T ) -> Self {
        Self{name: name.into(), inputs, outputs, time: time.into()}
    }
    pub fn isvalid(&self) -> bool {
        self.time.isvalid()
    }
}

impl RecipeBook {
    pub fn new<H: Into<HashMap<String, Recipe>>>(recipes: H) -> Self{
        Self{recipes: recipes.into()}
    }
    pub fn add_recipe(&mut self, recipe: Recipe) -> Option<Recipe>{
        self.recipes.insert(recipe.name.clone(), recipe)
    }
    pub fn add_from_list(&mut self, recipe_list: Vec<Recipe>) {
        // Add in new recipes
        for recipe in recipe_list {
            self.add_recipe(recipe);
        }
    }
    pub fn remove_recipe<S: Into<String>>(&mut self, recipe_name: S) -> Option<Recipe>{
        self.recipes.remove(&recipe_name.into())
    }
    // pub fn valid_recipe(&self, recipe_name: &String) -> bool {
    //     self.recipes.contains_key(recipe_name)
    // }
    pub fn get_recipe(&self, recipe_name: &String) -> Option<&Recipe> {
        self.recipes.get(recipe_name)
    }
    pub fn len(&self) -> usize {
        self.recipes.len()
    }
}

impl From<HashMap<String, Recipe>> for RecipeBook {
    fn from(recipes: HashMap<String, Recipe>) -> Self {
        Self {recipes}
    }
}

impl RecipeTime {
    pub fn isvalid(&self) -> bool {
        match self {
            Self::INVALID => false,
            _ => true
        }
    }
}

impl Default for RecipeTime {
    fn default() -> Self {
        RecipeTime::INVALID
    }
}
impl<F: Into<f32>> From<F> for RecipeTime {
    fn from(value: F) -> Self {
        let f: f32 = value.into();
        if f < 0. {
            RecipeTime::INVALID
        } else {
            RecipeTime::Time(f)
        }
    }
}