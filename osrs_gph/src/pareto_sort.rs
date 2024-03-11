use std::cmp::Ordering;

use prettytable::{Row, Table};

// type RowFormat = (String, i32, i32, f32, i32);
type Weights = Vec<f32>;

fn lin_scalarization(x: &Row, weights: &Weights) -> f32 {
    // Want to multiply weights with x, starting from 2nd element (i32)
    // Need to skip the first element of the Row (String)
    x.iter()
        .skip(1)
        .take(x.len() - 2)
        .map(|c| c.get_content().parse::<f32>().unwrap_or(1.))
        .zip(weights.iter())
        .map(|(a, b)| a * b)
        .sum()
}

fn ls_compare(x: &Row, y: &Row, weights: &Weights) -> Ordering {
    // TODO: Change None to Option?
    let x_none: bool = match x.get_cell(0) {
        Some(cell) => cell.get_content().contains("N/A"),
        None => true,
    };
    let y_none: bool = match y.get_cell(0) {
        Some(cell) => cell.get_content().contains("N/A"),
        None => true,
    };

    if x_none && y_none {
        // Want more if y is greater than x (y_len > x_len => More)
        return y.len().cmp(&x.len());
    }

    if x_none {
        return Ordering::Less;
    }
    if y_none {
        return Ordering::Greater;
    }

    let x_val = lin_scalarization(x, weights);
    let y_val = lin_scalarization(y, weights);
    // (x_val > y_val) - (x_val < y_val)
    // Want x_val > y_val (Opposite of x_val < y_val)
    x_val.total_cmp(&y_val)
}

fn optimal_sort(table: &Table, weights: &Weights, reverse: bool) -> Table {
    let _v_weights = compute_weights(weights); // Normalize
                                               // Return sorted based on ls_compare function
    let mut row_list = table.into_iter().collect::<Vec<&Row>>();
    row_list.sort_by(|a, b| ls_compare(a, b, weights));
    if reverse {
        row_list.reverse()
    };
    let mut output_table = Table::new();
    for row in row_list {
        output_table.add_row(row.clone());
    }
    output_table
}

#[must_use]
/// Normalize weights
pub fn compute_weights(weights: &Weights) -> Weights {
    let w_sum: f32 = weights.iter().map(|x| x.abs()).sum();
    weights.iter().map(|x| x / w_sum).collect()
}
