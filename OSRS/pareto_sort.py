"""
Pareto sort implementation.
"""

# Table headers are:
# Method,Loss/Gain,Total Loss/Gain,Time (h),GP/h
# (str),(int),(int),(float),(int)

# The table is a list of tuples
# Each tuple contains
#(
#     recipe, # Method name
#     margin, # Loss/Gain, this can be positive (gain) or negative (loss)
#     amount*margin, # Total Loss/Gain, ditto
#     total_time_h, # Time (h), only positive
#     GP_h # GP/h, calculated by (margin/time), where time is known
# )

# Import converting python 2 cmp to python 3 keys
from functools import cmp_to_key


RowFormat = tuple[str,int,int,float,int]
TableFormat = list[RowFormat]

# Want to sort by the conditions:
#   - Maximize Total Loss/Gain
#   - Minimize Time (h)
# x: tuple[str,int,int,float,int]
def lin_scalarization(x, weights): # In python 2 cmp form
    """Linear scalarization function."""
    return sum(w*x[i] for i,w in enumerate(weights, start=1))

def lin_scalarization_cmp(x,y, weights): # In python 3 key form
    """Comparison function for sort."""
    # If both are None or "N/A" string then it doesn't matter which order
    # Check if None or "N/A" string are in x
    x_none = (None in x) or ("N/A" in x)
    y_none = (None in y) or ("N/A" in y)

    if x_none and y_none:
        # Sort by recipe name length
        x_len = len(x[0])
        y_len = len(y[0])
        if x_len < y_len:
            return 1
        if x_len > y_len:
            return -1
        return 0
    # Otherwise, check if only one is None or "N/A" string
    if x_none:
        return -1
    if y_none:
        return 1


    # Otherwise, compare
    x_val = lin_scalarization(x,weights)
    y_val = lin_scalarization(y,weights)
    # -1*(x_val < y_val) + 1*(x_val > y_val) + 0*(x_val == y_val)
    return (x_val > y_val) - (x_val < y_val)


def optimal_sort_inner(table: TableFormat, weights, reverse) ->  TableFormat:
    """Internals of optimal sort. Actually computes sort."""
    # Use linear scalarization to sort
    return sorted(table, key=cmp_to_key(lambda x,y: lin_scalarization_cmp(x,y,weights)), reverse=reverse)


# Find ranking differnces before and after sorting
# i.e.: if a row has increased by 2 indexes, then it has moved *down* by 2
# Use recipe name (first value) as key
def find_ranking_diffs(before:TableFormat, after:TableFormat) -> dict[str,int]:
    """Find ranking differences before and after sorting."""
    # Create dict of recipe name to index
    before_dict = {row[0]:i for i,row in enumerate(before)}
    after_dict = {row[0]:i for i,row in enumerate(after)}

    # Find differences
    diffs = {recipe:(after_dict[recipe] - before_dict[recipe]) for recipe in before_dict}

    return diffs

def compare_rankings(before: dict[str,int], after: dict[str,int]) -> dict[str,int]:
    """Compare relative positions of different ranking results."""
    # Find differences
    return {recipe:(after[recipe] - before[recipe]) for recipe in before}

def compute_weights(weights:list[float]) -> list[float]:
    """Normalize weights."""
    # Weights must sum to 1; rescaling
    w_sum = sum(abs(w) for w in weights)
    return [w/w_sum for w in weights]

def optimal_sort(table: TableFormat, weights, reverse=False) -> TableFormat:
    """Pareto sort."""
    weights = list(weights)
    weights = compute_weights(weights)
    return optimal_sort_inner(table, weights, reverse)

def main():
    """Main function."""
    # Load profitDetails_list.txt
    with open("profitDetails_list.txt", "r", encoding="utf-8") as f:
        table = f.read() # Only one line
        table = eval(table) # Convert to list, SAFE
        # Sort
        # margins, total_margins, time, gp_h
        weights1 = compute_weights([0,0,0,1]) # Just gp/h

        weights2 = compute_weights([0,1,-3,0]) # Gain, and time
        custom_table1 = optimal_sort(table, weights2, reverse=True)
        custom_table2 = optimal_sort(table, weights1, reverse=True)

        ranking_diffs1 = find_ranking_diffs(table, custom_table1)
        ranking_diffs2 = find_ranking_diffs(table, custom_table2)

        ranking_comparison = compare_rankings(ranking_diffs1, ranking_diffs2)
        print(ranking_comparison)
if __name__ == "__main__":
    main()
