use std::{path::Path, collections::HashMap};

use crate::{logging::Logging, item_search::{ItemSearch, RecipeTime, Recipe, Item}, convenience::{floor, f_round}};

pub struct PriceHandle<'a, 'b, 'c, S: AsRef<Path>> {
    pub all_items: Logging<'a, ItemSearch<'a, 'b, 'c, S>>,
    pub coins: i32,
    pub pmargin: f32
}

impl<'a, 'b, 'c, S: AsRef<Path>> PriceHandle<'a, 'b, 'c, S>  {
    pub fn new(all_items: Logging<'a, ItemSearch<'a, 'b, 'c, S>>,
    coins: i32,
    pmargin: f32) -> Self {
        Self { all_items, coins, pmargin}
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
    pub fn total_price(price_details: &[(f32, f32)]) -> i32{
        // total price for each item is price * quantity
        let total: f32 = price_details.iter()
            .map(|t| t.0*t.1)
            .sum();
        floor(total)
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    /// Calculates the total recipe time, parsing invalid times
    pub fn recipe_time_h(recipe: &Recipe, number: i32, margin: i32, total_margin: bool) -> (Option<f32>, Option<i32>) {
        if let RecipeTime::Time(t) = recipe.time {
            let time_h: f32 = t / (60. * 60.);
            let total_time_h:f32 = f_round(number as f32*time_h, 2);
            
            let gp_h = if total_margin {
                floor(margin as f32 / total_time_h)
            } else {
                floor(margin as f32 / time_h)
            };

            return (Some(total_time_h), Some(gp_h))
        
        }
        (None, None)
        
    }

    pub fn item_list_prices<I: IntoIterator<Item = (Item, f32)>>(item_list: I, price_type: bool) -> HashMap<Item, (Option<i32>, f32)> {
        item_list.into_iter()
            .map(|(i, q)| {
                let price = i.price(price_type);
                (i, (price, q))
            }
            )
            .collect()

    }

}