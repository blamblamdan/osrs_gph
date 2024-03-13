use crate::{
    convenience::{f_round, floor},
    item_search::{Item, Recipe, RecipeTime},
    logging::{LogItemSearch, LogRecipeBook},
};

use std::{collections::HashMap, path::Path, fmt::Display};

pub struct PriceHandle<'a, S: AsRef<Path>> {
    pub all_items: LogItemSearch<'a, 'a, S>,
    pub recipe_list: LogRecipeBook<'a>,
    pub coins: i32,
    pub pmargin: f32,
}

impl<'a, S: AsRef<Path> + Display> PriceHandle<'a, S> {
    pub fn new(all_items: LogItemSearch<'a, 'a, S>, recipe_list: LogRecipeBook<'a>, coins: i32, pmargin: f32) -> Self {
        Self {
            all_items,
            recipe_list,
            coins,
            pmargin,
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn apply_tax(profit: i32) -> i32 {
        if profit < 100 {
            profit
        } else {
            let tax = 5_000_000.min(floor(profit as f32 * 0.01));
            profit - tax
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn total_price(price_details: &[(i32, f32)]) -> i32 {
        // total price for each item is price * quantity
        let total: f32 = price_details.iter().map(|t| t.0 as f32 * t.1).sum();
        floor(total)
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    /// Calculates the total recipe time, parsing invalid times
    pub fn recipe_time_h(
        recipe: &Recipe,
        number: i32,
        margin: i32,
        total_margin: bool,
    ) -> (Option<f32>, Option<i32>) {
        if let RecipeTime::Time(t) = recipe.time {
            let time_h: f32 = t / (60. * 60.);
            let total_time_h: f32 = f_round(number as f32 * time_h, 2);

            let gp_h = if total_margin {
                floor(margin as f32 / total_time_h)
            } else {
                floor(margin as f32 / time_h)
            };

            return (Some(total_time_h), Some(gp_h));
        }
        (None, None)
    }

    /// (Item, (Price, Quantity))
    pub fn item_list_prices<I: IntoIterator<Item = (Item, f32)>>(
        item_list: I,
        price_type: bool,
    ) -> HashMap<Item, (Option<i32>, f32)> {
        item_list
            .into_iter()
            .map(|(i, q)| {
                let price = i.price(price_type);
                (i, (price, q))
            })
            .collect()
    }

    /// # Panics
    /// When item price does not exist.
    pub fn item_list_prices_unchecked<I: IntoIterator<Item = (Item, f32)>>(
        item_list: I,
        price_type: bool,
    ) -> HashMap<Item, (i32, f32)> {
        item_list
            .into_iter()
            .map(|(i, q)| {
                let price = i.price(price_type).unwrap();
                (i, (price, q))
            })
            .collect()
    }

    pub fn parse_item_list(&self, items: &HashMap<String, f32>) -> Option<Vec<(Item, f32)>> {
        let filtered_items: Vec<_> = items
            .iter()
            .filter_map(|(item_name, &quantity)|
                self.all_items
                .item_by_name(item_name)
                .map(|item| (item.clone(), quantity))
            ).collect();
        if items.len() == filtered_items.len() {
            Some(filtered_items)
        } else {
            None
        }
    }

}
