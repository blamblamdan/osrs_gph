use crate::{logging::LogFileIO, data_types::PriceDatum};

use std::{collections::HashMap, fmt::Debug, hash::Hash, path::Path};

use serde::{de::Visitor, Deserialize};

#[derive(Debug, Deserialize, Clone)]
pub struct Item {
    pub name: String,    // TODO: Consider switching to &str if not needed.
    pub item_id: String, // i32
    pub item_prices: PriceDatum,
}

// Required by HashMaps
impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.item_id == other.item_id
    }
}
impl Eq for Item {}
impl Hash for Item {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.item_id.hash(state);
    }
}

pub struct ItemSearch<'a, S: AsRef<Path>> {
    // Curse of logging wrapper...
    pub price_data_handler: LogFileIO<'a, S>,
    pub id_to_name_handler: LogFileIO<'a, S>,
    pub name_to_id_handler: LogFileIO<'a, S>,
    pub items: HashMap<String, Item>,
    pub name_to_id: HashMap<String, String>,
    pub id_to_name: HashMap<String, String>,
}

#[derive(Debug)]
// #[serde(untagged)]
#[derive(Default, Clone)]
pub enum RecipeTime {
    Time(f32),
    #[default]
    INVALID,
}

impl<'de> Deserialize<'de> for RecipeTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RecipeTimeVisitor;
        impl<'de> Visitor<'de> for RecipeTimeVisitor {
            type Value = RecipeTime;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("f32 or no field")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RecipeTime::INVALID)
            }
            #[allow(clippy::cast_possible_truncation)]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok((v as f32).into())
            }
            #[allow(clippy::cast_precision_loss)]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok((v as f32).into())
            }
            #[allow(clippy::cast_precision_loss)]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok((v as f32).into())
            }
        }
        let de = deserializer.deserialize_f32(RecipeTimeVisitor);
        if let Err(e) = de {
            if e.to_string().contains("missing field") {
                Ok(RecipeTime::INVALID)
            } else {
                Err(e)
            }
        } else {
            de
        }
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Recipe {
    pub name: String,
    pub inputs: HashMap<String, f32>,
    pub outputs: HashMap<String, f32>,
    pub time: RecipeTime,
}

#[derive(Debug, Deserialize, Default)]
pub struct RecipeBook {
    pub recipes: HashMap<String, Recipe>,
}

impl<'a, S: AsRef<Path> + std::fmt::Display> ItemSearch<'a, S> {
    pub fn new(
        price_data_handler: LogFileIO<'a, S>,
        id_to_name_handler: LogFileIO<'a, S>,
        name_to_id_handler: LogFileIO<'a, S>,
        items: HashMap<String, Item>,
    ) -> Self {
        Self {
            price_data_handler,
            id_to_name_handler,
            name_to_id_handler,
            items, // Using Item Name(String)=>Item(Object)
            name_to_id: HashMap::new(),
            id_to_name: HashMap::new(),
        }
    }
    pub fn name_from_id(&self, item_id: &String) -> Option<&String> {
        self.id_to_name.get(item_id)
    }
    pub fn id_from_name(&self, item_name: &String) -> Option<&String> {
        self.name_to_id.get(item_name)
    }
    pub fn item_by_name(&self, item_name: &String) -> Option<&Item> {
        self.items.get(item_name) 
    }  
    pub fn item_by_id(&self, item_id: &String) -> Option<&Item> {
        if let Some(item_name) = self.name_from_id(item_id) {
            self.item_by_name(item_name)
        } else {
            None
        }
    }
}

impl Item {
    #[must_use]
    pub fn new(name: String, id: String, price_data: PriceDatum) -> Self {
        Self {
            name,
            item_id: id,
            item_prices: price_data,
        }
    }
    #[must_use]
    pub fn invalid_data(&self) -> bool {
        self.item_prices.invalid_data()
    }
    #[must_use]
    pub fn price(&self, high_price: bool) -> Option<i32> {
        if high_price {
            self.item_prices.high
        } else {
            self.item_prices.low
        }
    }
    #[must_use]
    pub fn price_tuple(&self) -> HashMap<String, Option<i32>> {
        HashMap::from_iter([
            ("high".to_owned(), self.item_prices.high),
            ("low".to_owned(), self.item_prices.low),
        ])
    }
}

impl Recipe {
    pub fn new<S: Into<String>, T: Into<RecipeTime>>(
        name: S,
        inputs: HashMap<String, f32>,
        outputs: HashMap<String, f32>,
        time: T,
    ) -> Self {
        Self {
            name: name.into(),
            inputs,
            outputs,
            time: time.into(),
        }
    }
    #[must_use]
    pub fn isvalid(&self) -> bool {
        self.time.isvalid()
    }
}

impl RecipeBook {
    pub fn new<H: Into<HashMap<String, Recipe>>>(recipes: H) -> Self {
        Self {
            recipes: recipes.into(),
        }
    }
    pub fn add_recipe(&mut self, recipe: Recipe) -> Option<Recipe> {
        self.recipes.insert(recipe.name.clone(), recipe)
    }
    pub fn add_from_list(&mut self, recipe_list: Vec<Recipe>) {
        // Add in new recipes
        for recipe in recipe_list {
            self.add_recipe(recipe);
        }
    }
    pub fn remove_recipe<S: Into<String>>(&mut self, recipe_name: S) -> Option<Recipe> {
        self.recipes.remove(&recipe_name.into())
    }
    #[must_use]
    pub fn get_recipe(&self, recipe_name: &String) -> Option<&Recipe> {
        self.recipes.get(recipe_name)
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.recipes.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
}

impl From<HashMap<String, Recipe>> for RecipeBook {
    fn from(recipes: HashMap<String, Recipe>) -> Self {
        Self { recipes }
    }
}

impl RecipeTime {
    #[must_use]
    pub fn isvalid(&self) -> bool {
        !matches!(self, Self::INVALID)
    }
}

impl ToString for RecipeTime {
    fn to_string(&self) -> String {
        match self {
            RecipeTime::Time(t) => format!("{t}"),
            RecipeTime::INVALID => String::new()
        }
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
