"""
Started: 2023-12-25
Goal: Rewrite the old profit_margins script using OOP (Avoiding global variables).
"""

import json
import math

from pathlib import Path
from types import NoneType
from typing import IO, Any, BinaryIO, Callable, Iterable, Iterator, ValuesView, cast

import logging
from logging import warning as log_warning
from math import floor as m_floor
import tomli
import requests
from tabulate import tabulate
from pareto_sort import optimal_sort


# w means refresh each program run
logging.basicConfig(filename='runtime.log', filemode='w', encoding='utf-8', level=logging.DEBUG)

def create_arg_kwargs(*args, **kwargs):
    """I'm not writing these out manually."""
    return args, kwargs

class FileIO:
    """File handling class."""
    def __init__(self, filename: str) -> None:
        self.filename = filename

    def exists(self) -> bool:
        """Check if the file exists."""
        # Path might be relative to the current working directory
        return Path(self.filename).is_file()

    def raw_load(self, *fileargs, **filekwargs) -> IO|None:
        """Load file object."""
        if self.exists():
            try:
                return open(self.filename, *fileargs, **filekwargs)
            except OSError as e:
                logging.error("Failed to load file '%s'", self.filename)
                logging.error("Error: %s", e)
                return None
        else:
            print(f"File '{self.filename}' does not exist.")
            return None

    def raw_read(self, *fileargs, mode = "r", **filekwargs) -> str|None:
        """Read file as string."""
        # Might not work?
        f = self.raw_load(mode=mode, *fileargs, **filekwargs)
        if f is not None:
            return f.read()
        else:
            return None

    def read(self, *jsonargs,  fileargs: tuple[Any, ...]|None=None, filekwargs: dict[str, Any]|None = None,  **jsonkwargs) -> dict|None:
        """Read file, with JSON parsing."""
        if fileargs is None:
            fileargs = ()
        if filekwargs is None:
            filekwargs = {}
        f = self.raw_load(*fileargs, **filekwargs)

        if f is not None:
            try:
                with f: # Close after reading
                    return json.load(f, *jsonargs, **jsonkwargs)
            except ValueError: # TODO: Change to JSONDecodeError?
                logging.error("Failed to load file '%s'", self.filename)
                return None
        else:
            return None

    def write(self, data: Any, *fileargs, mode: str = "w+", indent: int = 4, **filekwargs) -> Any|None:
        """Write data to file, with JSON parsing."""
        if self.exists():
            logging.warning("Overwriting file '%s'", self.filename)

        f = self.raw_load(mode=mode, *fileargs, **filekwargs)
        if f is not None:
            try:
                with f: # Close after writing
                    json.dump(data, f, indent=indent, *fileargs, **filekwargs)
                    return data
            # TODO: OSError and JSONEncodeError and any other errors?
            except Exception as e:
                logging.error("Failed to write to file '%s'", self.filename)
                logging.critical("Error: %s", e)
                return None
        else:
            return None

    def delete(self) -> bool:
        """Delete the file."""
        if self.exists():
            try:
                Path(self.filename).unlink()
                return True
            except OSError as e:
                logging.error("Failed to delete file '%s'", self.filename)
                logging.error("%s", e)
                return False

        else:
            logging.error("File '%s' does not exist.", self.filename)
            return False

class API:
    """For interacting with the OSRS GE API."""
    def __init__(self, url, auth_headers):
        self.url = url
        self.headers = auth_headers


    def request(self, endpoint: str|bytes, callback: Callable, headers:dict|None=None) -> Any|None:
        """Makes a request to the API and returns the JSON response."""
        with self(endpoint, headers) as r:
            logging.info("Request sent to %s", endpoint)
            try:
                r.raise_for_status()
            except requests.HTTPError as e:
                error_msg = f"Bad response: {e}"
                logging.critical(error_msg)
                raise requests.HTTPError(error_msg)
            else:
                return callback(r.json()) # Success

    # Raw API requests
    def __call__(self, endpoint: str|bytes, headers:dict|None=None) -> requests.Response:
        if headers is None:
            headers = self.headers
        else:
            # Merge headers, prioritising the passed headers
            headers = {**self.headers, **headers}
        return requests.get(url=self.url+endpoint, headers=headers, timeout=6)

    def __repr__(self) -> str:
        return f"API(url={self.url}, headers={self.headers})"

class Item:
    """
    Represents a single item in the game.
    """
    def __init__(self, item_id: int, name: str, high: int, high_time: int, low: int, low_time: int):
        """Collates all data from the different API results into a single object.
        id: int - The item's ID
        name: str - The item's name
        high: int - The item's high price
        highTime: int - The time the high price was last updated
        low: int - The item's low price
        lowTime: int - The time the low price was last updated
        duplicate: bool - Whether the item name is a duplicate of another item name
        """
        self.item_id = item_id
        self.name = name
        self.high = high
        self.high_time = high_time
        self.low = low
        self.low_time = low_time

    def valid_data(self) -> bool:
        """Checks whether the data is valid."""
        # If any is None
        return not (self.high is None or self.high_time is None or self.low is None or self.low_time is None)

    def __repr__(self) -> str:
        return f"Item(id={self.item_id}, name={self.name}, high=({self.high} @ {self.high_time}), low=({self.low} @ {self.low_time}))"

class ItemSearch:
    """
    Contains all items; for lookup purposes.
    """
    def __init__(self, items: dict|None, price_handler: FileIO, id_to_name_handler: FileIO, name_to_id_handler: FileIO, ignore: Iterable[str]) -> None:
        # Same order as in config.toml
        self.price_data_handler = price_handler
        self.id_to_name_handler = id_to_name_handler
        self.name_to_id_handler = name_to_id_handler

        self.id_to_name = self.load_id_to_name()
        self.name_to_id = self.load_name_to_id()

        self.items = {
            # "item_id": Item
            # Add value of a coin (1gp)
            "617" : Item(617, "Coins", 1, 0, 1, 0)
        }

        self.ignored_items = ignore

        if items is None:
            self.populate_items()
        else:
            self.items = items
        self.remove_ignored_items()

    # FileIO stuff
    def load_id_to_name(self) -> dict[str, str]:
        """Loads the ID to name dictionary."""
        id_to_name = self.id_to_name_handler.read()
        if id_to_name is not None:
            return id_to_name
        else:
            return {}
    def load_name_to_id(self) -> dict[str, str]:
        """Loads the name to ID dictionary."""
        name_to_id = self.name_to_id_handler.read()
        if name_to_id is not None:
            return name_to_id
        else:
            return {}
    def load_item_data(self) -> dict[str, dict[str, int]]:
        """
        Loads all item price data from the API.
        """
        # Load all items
        items = self.price_data_handler.read()
        if items is not None:
            item_data = items.get("data")
            if item_data is not None:
                return item_data
        return {}

    # Parsing loaded data
    def populate_items(self) -> None:
        """
        Populates the items dict with all items.
        """
        items = self.load_item_data()

        # Check whether the key used "high" or "avgHighPrice", same for "low"
        # This is same for all items
        if "avgHighPrice" in items["2"]:# "2" is the ID for "Cannonball"
            high_key_name = "avgHighPrice"
            high_time_name = "highPriceVolume"
            low_key_name = "avgLowPrice"
            low_time_name = "lowPriceVolume"
        else:
            high_key_name = "high"
            high_time_name = "highTime"
            low_key_name = "low"
            low_time_name = "lowTime"

        # Create an Item object for each item
        for item_id, price_values in items.items():
            item_name = self.id_to_name.get(item_id, -1)
            if item_name == -1:
                log_warning(f"Item ID {item_id} not found in id_to_name.json")
                continue

            item = Item(
                int(item_id),
                str(item_name),
                price_values[high_key_name],
                price_values[high_time_name],
                price_values[low_key_name],
                price_values[low_time_name]
            )
            # Check if the item data is valid
            if not item.valid_data():
                # Logging would spam the console
                # logging.warning(f"Full item data for {item.name} not found.")
                continue

            self.items[item_id] = item
    # Lookup converters
    def item_name_to_id(self, item_name: str) -> str|NoneType:
        """Converts an item name to an item ID."""
        return self.name_to_id.get(item_name, None)
    def item_id_to_name(self, item_id: str) -> str|NoneType:
        """Converts an item ID to a item name."""
        return self.id_to_name.get(item_id, None)
    def item_s_to_id(self, item_s: str) -> str|NoneType:
        """Converts a string to an item ID."""
        # First check if it's an item ID
        if item_s in self.id_to_name.keys():
            return item_s
        # Must be a name
        return self.name_to_id.get(item_s, None)
    # Checker
    def isvalid_item(self, item_s: str) -> bool:
        """Check if item id exists."""
        return self.item_s_to_id(item_s) is not None
    def item_data_exists(self, item_s: str) -> bool:
        """Checks whether the item data exists."""
        item_id = self.item_s_to_id(item_s)
        if item_id is None:
            return False
        return item_id in self.items

    # Raw dictionary access
    def get_item_by_id(self, item_id: str) -> Item|NoneType:
        """Retrive item object by item ID."""
        if item_id not in self.items.keys():
            logging.warning("Item not found: %s", item_id)
            return None
        return self.items[item_id]

    # Alternative - checked
    def get_item_by_name(self, item_name: str) -> Item|NoneType:
        """Get item object by item name."""
        item_id = self.item_name_to_id(item_name)
        if item_id is None:
            logging.warning("Item not found: %s", item_name)
            return None
        return self.items[item_id]

    def remove_ignored_items(self, ignore: Iterable[str]|None = None) -> None:
        """Removes all items in the ignore list."""
        logging.debug("Removing ignored items.")
        if ignore is None:
            ignore = self.ignored_items
        else:
            self.ignored_items = ignore
        before_len = len(self.items)

        for item_s in ignore:
            item_id = self.item_s_to_id(item_s)
            if item_id is None:
                log_warning("Invalid item to remove: %s", item_s)
                continue
            self.items.pop(item_id)

        after_len = len(self.items)
        logging.debug("Removed %d ignored items.", before_len-after_len)

class ItemList:
    """Array of item objects, with their quantities."""
    def __init__(self, items: dict|None = None) -> None:
        self.items: dict[Item, float] = {} if items is None else items

    # Add items, with alternative methods (Overwrites)
    def add_item(self, item: Item, quantity: float):
        """Adds an item object to the list."""
        self.items[item] = quantity
    def add_item_tuple(self, item_tuple: tuple[Item, float]):
        """Adds an item and its quantity to the list."""
        self.items[item_tuple[0]] = item_tuple[1]
    def add_item_by_str(self, item_s: str, quantity: float, item_search: ItemSearch) -> bool:
        """Adds an item to the list  by its name or id string."""
        ## Want to check if item price data exists or not; if it doesn't, then don't add it
        item_id = item_search.item_s_to_id(item_s)
        if item_id is None:
            logging.warning("Item not found: %s", item_s)
            return False
        if not item_search.item_data_exists(item_id):
            logging.warning("Item data not found: %s", item_s)
            return False

        # TODO: Raw access; this is already checked above
        item = item_search.get_item_by_id(item_id)
        if item is None:
            logging.warning("Item not found: %s", item_id)
            return False

        self.items[item] = quantity
        return True


    # Remove item (Singular)
    def remove_item(self, item: Item):
        """Removes an item from the list."""
        self.items.pop(item)
    def __repr__(self) -> str:
        return f"{self.items}"

class Recipe:
    """Definition of a recipe, including inputs, outputs, and time."""
    def __init__(self, name: str, inputs: ItemList, outputs: ItemList, time: float):
        # If time is not known, it will be set to -1.
        self.name = name
        self.inputs = inputs
        self.outputs = outputs
        self.time = time

    def isvalid(self) -> bool:
        """Checks whether the recipe was initialised properly."""
        return self.time != -2 # TODO: Replace with enum in python 3.12

    def __repr__(self) -> str:
        return f"{self.name} ({self.inputs} -> {self.outputs})"
    def __str__(self) -> str:
        title = f"\nRecipe '{self.name}':\n"

        # Display inputs on new lines
        inputs = "\n".join(
        f"- {item.name}: {quantity}" for item, quantity in self.inputs.items.items()
        )
        outputs = "\n".join(
        f"- {item.name}: {quantity}" for item, quantity in self.outputs.items.items()
        )

        return title + "Inputs:\n" + inputs + "\nOutputs:\n" + outputs

class RecipeBook:
    """Collection of all recipes."""
    def __init__(self, available_items: ItemSearch, recipe_fp: str, recipes_list: dict|None = None) -> None:
        self.recipes = {}
        if recipes_list is not None:
            for recipe in recipes_list.values():
                self.add_recipe(recipe)
            # Remove template
            self.remove_recipe_by_name("Template") # It exists at this point
        else:
            self.load_default_recipes(available_items, recipe_fp)
    def load_default_recipes(self, all_items: ItemSearch, recipe_path:str) -> None:
        """Loads the default recipes from the recipes Json file."""
        recipes_fio = FileIO(recipe_path)
        recipes_uf = recipes_fio.read(object_hook=lambda x: recipe_hook(x, all_items))
        if recipes_uf is None:
            raise ValueError("Failed to load recipes.")

        unfiltered_len = len(recipes_uf)
        # Filter out invalid recipes; using the isvalid method
        # Log any invalid recipes
        recipes = {}
        for recipe_name, recipe in recipes_uf.items():
            if not recipe.isvalid():
                log_warning("Skipping recipe: %s", recipe)
                continue
            recipes[recipe_name] = recipe

        filtered_len = len(recipes)
        logging.debug("Filtered out %d invalid recipes.", unfiltered_len - filtered_len)

        for recipe in recipes.values():
            self.add_recipe(recipe)
        # Remove template
        self.remove_recipe_by_name("Template")
        logging.debug("Loaded %d recipes.", len(self.recipes))

    def add_recipe(self, recipe: Recipe):
        """Adds a recipe to the recipe book."""
        self.recipes[recipe.name] = recipe
    def remove_recipe(self, recipe: Recipe):
        """Removes a recipe from the recipe book."""
        self.recipes.pop(recipe.name)
    def remove_recipe_by_name(self, recipe_name: str):
        """Removes a recipe from the recipe book by recipe name."""
        self.recipes.pop(recipe_name)

    def valid_recipe(self, recipe_name: str) -> bool:
        """Checks whether a recipe is in the recipe book."""
        return recipe_name in self.recipes

    def get_recipe(self, recipe_name: str) -> Recipe|NoneType:
        """Retrieves a recipe from the recipe book."""
        if self.valid_recipe(recipe_name):
            return self.recipes[recipe_name]
        else:
            logging.error("Invalid recipe name: %s", recipe_name)
            return None

    def __repr__(self) -> str:
        return f"{self.recipes.keys()}"

class PriceAPI(API):
    """For interacting with the OSRS GE API."""
    def __init__(self, url, auth_headers, price_data_IO: FileIO, timespan: str = "latest"):
        super().__init__(url, auth_headers)
        self.prices_io = price_data_IO
        self.ts = timespan

    def get_all_prices(self, timespan:str|None = None) -> dict|None:
        """Retrieves and writes all item prices to the file."""
        ts_map: set[str] = {
            "latest",
            "5m",
            "1h"
        }
        if timespan is None:
            ts:str = self.ts # Timespan
        else:
            ts:str = timespan
        if not ts in ts_map:
            logging.error("Timespan %s not supported.", ts)
            return None
        if not ts.startswith("/"):
            ts = "/"+ts

        try:
            return self.request(ts, self.prices_io.write) # Please work
        except OSError as e:
            logging.critical("Failed to retrieve prices.")
            logging.critical("Error: %s", e)
            return None

class PriceHandle:
    """Processing of price data *in memory*."""
    # Take in ItemSearch object
    def __init__(self, item_search: ItemSearch, recipe_book: RecipeBook, coins: int, percent_margin: float, weights:tuple) -> None:
        self.all_items = item_search
        self.recipe_book = recipe_book
        self.coins = coins
        self.pm = percent_margin
        self.weights = weights

    def item_price_data(self, item: Item) -> dict[str, int]:
        """Returns the full price data of an item."""
        return { # Ignore the time for now 2023-12-26
            "high": item.high,
            "low": item.low
        }

    def item_price(self, item: Item, price_type: bool) -> int:
        """
        Gets the price of an item. 
        True for high (Buying), False for low (Selling).
        """
        if price_type is True:
            return item.high
        elif price_type is False:
            return item.low
        else:
            raise ValueError(f"Invalid price type: {price_type}")

    def item_list_prices(self, items:ItemList, price_type: bool) -> dict[Item, tuple[int, float]]:
        """Returns price and quantity of each item in the list."""
        # (Item: price, amount)
        return {item: (self.item_price(item, price_type), quantity) for item, quantity in items.items.items()}

    def recipe_price_overview(self, recipe_s: str) -> tuple[int,int,int,float]|None:
        """Returns the profit margins of a recipe."""
        # logging.debug(f"Calculating price overview for {recipe_s}")
        recipe = self.recipe_book.get_recipe(recipe_s)
        if recipe is None:
            return None
        input_details = self.item_list_prices(recipe.inputs, True)
        output_details = self.item_list_prices(recipe.outputs, False)
        cost = self.total_price(input_details.values())
        revenue = self.total_price(output_details.values())
        revenue = self.apply_tax(revenue)
        time = recipe.time

        return cost, revenue, revenue-cost, time

    def specific_recipe_details(self, recipe_s: str) -> dict[str, dict[str,Any]]|None:
        """Returns the details of a specific recipe."""
        # logging.debug(f"Calculating specific recipe details for {recipe_s}")
        # Get recipe
        recipe = self.recipe_book.get_recipe(recipe_s)
        if recipe is None:
            return None
        # Get initial price details for both inputs and outputs
        overview = self.recipe_price_overview(recipe_s)
        if overview is None:
            return None

        input_total, output_total, margin, time = overview

        # Get the price details for both inputs and outputs
        input_details = self.item_list_prices(recipe.inputs, True)

        output_details = self.item_list_prices(recipe.outputs, False)

        # Calculate new margins with percent margin applied.
        # Increase buy price, decrease sell price
        input_details_pm = {item: (math.floor(price*(1+self.pm/100)), quantity) for item, (price, quantity) in input_details.items()}
        output_details_pm = {item: (math.floor(price*(1-self.pm/100)), quantity) for item, (price, quantity) in output_details.items()}

        # Calculate new overview values for percent margin
        input_total_pm = self.total_price(input_details_pm.values())
        output_total_pm = self.total_price(output_details_pm.values())
        # Apply tax to output
        output_total_pm = self.apply_tax(output_total_pm)

        pmargin = output_total_pm - input_total_pm

        # Calculate number of recipes which can be made with current coins
        amount = max(0,math.floor(self.coins/input_total))
        amount_pm = max(0,math.floor(self.coins/input_total_pm))

        # Calculate total money made/lost
        total_margin = margin*amount
        total_pmargin = pmargin*amount_pm

        total_time, gp_h = self.recipe_time(margin, time, amount)
        total_time_pm, gp_h_pm = self.recipe_time(pmargin, time, amount_pm)

        basic_details = {
            "input_details": input_details,
            "output_details": output_details,
            "input_total": input_total,
            "output_total": output_total,
            "margin": margin,
            "amount": amount,
            "total_margin": total_margin,
            "total_time": total_time,
            "GP/h": gp_h
        }
        pm_details = {
            "input_details": input_details_pm,
            "output_details": output_details_pm,
            "input_total": input_total_pm,
            "output_total": output_total_pm,
            "margin": pmargin,
            "amount": amount_pm,
            "total_margin": total_pmargin,
            "total_time": total_time_pm,
            "GP/h": gp_h_pm
        }
        overview = {
            "basic_details": basic_details,
            "pm_details": pm_details
        }
        return overview

    def all_recipe_price_overview(self, sort_by_u: tuple[float,float,float,float]|None=None, profiting=True, show_hidden=False, reverse=True):
        """Returns the basic profit margins of all recipes."""
        logging.info("Calculating all recipe price overview.")
        if sort_by_u is None:
            sort_by = self.weights
        else:
            sort_by = sort_by_u
        # Get dictionary of recipe names, and their price overviews
        all_recipe_prices = {recipe_name: self.recipe_price_overview(recipe_name) for recipe_name in self.recipe_book.recipes.keys()}

        # Remove none entries, and assert type
        # Shouldn't change anything, but just in case
        all_recipe_prices = {recipe_name: overview for recipe_name, overview in all_recipe_prices.items() if overview is not None}

        all_recipe_details = []

        for (recipe_s, [recipe_cost, _, margin, time]) in all_recipe_prices.items():
            margin = m_floor(margin)

            cant_afford = self.coins < recipe_cost
            recipe_doesnt_profit = margin <= 0

            if  (cant_afford & ~show_hidden) | (recipe_doesnt_profit & profiting & ~show_hidden):
                # Don't show *anything*
                continue

            # N/A conditions
            if (cant_afford & show_hidden) | (recipe_doesnt_profit & profiting & show_hidden):
                all_recipe_details.append(
                    (
                        recipe_s,
                        None,
                        None,
                        None,
                        None
                    )
                )
                continue

            # Normal satisfies
            # (~recipe_doesnt_profit & ~cant_afford) | (~profiting & ~cant_afford)

            amount = m_floor(self.coins/recipe_cost) # pylint: disable=W8201

            all_recipe_details.append(
                (
                    recipe_s,
                    margin,
                    amount*margin,
                    *self.recipe_time(margin, time, amount)
                )
            )
        all_recipe_details = optimal_sort(all_recipe_details, sort_by, reverse=reverse)
        return all_recipe_details

    @staticmethod
    def recipe_time(margin, time, amount, total_margin=False):
        """Calculates the total time and gp/h for a recipe."""
        match time:
            # TODO: Change to enum in python 3.12
            case -1:
                total_time_h = None
                gp_h = None
            case t:
                time_h = t / (60*60)
                total_time_h = round(amount*time_h, 2)

                if total_margin:
                    gp_h = m_floor(margin / total_time_h)
                else:
                    gp_h = m_floor(margin / time_h)

        return total_time_h, gp_h

    @staticmethod
    def total_price(price_details: Iterable[tuple[int, float]]) -> int:
        """Returns the total price of the items."""
        # total price for each item is price * quantity
        return math.floor(sum(item[0]*item[1] for item in price_details))

    @staticmethod
    def apply_tax(profit: int) -> int:
        """Applies the tax to the profit."""
        return profit if profit < 100 else (profit - min(5_000_000, math.floor(profit*0.01)))


class ResultWriter(FileIO):
    """For writing the calculated price results to files."""
    def __init__(self, price_io: PriceHandle, display_number: int, show_top: int, profiting:bool, show_hidden:bool, reverse:bool, optimal:str, lookup:str) -> None:
        self.price_io = price_io
        self.dispn = display_number
        self.topn = show_top if display_number == 0 else min(show_top, display_number)
        self.profiting = profiting
        self.show_hidden = show_hidden
        self.reverse = reverse

        self.overview = optimal
        self.specific = lookup
        super().__init__(self.overview) # Set as this for now

        self.item_headers = ("Item", "Amount", "To Buy", "Price", "Total Price", "Total Time (h)", "Profit/Recipe Time (GP/h)")
        self.method_headers=  ("Method","Loss/Gain","Total Loss/Gain", "Time (h)", "GP/h")
        self.blank_line = [None]*len(self.item_headers)
        self.percent_str = f"{price_io.pm:.2f}%"

    def switch_to_overview(self):
        """Switches the filename to the overview file."""
        logging.debug("Switching to %s", self.overview)
        self.filename = self.overview
    def switch_to_lookup(self):
        """Switches the filename to the lookup file."""
        logging.debug("Switching to %s", self.specific)
        self.filename = self.specific

    def truncate_results(self, results: list|tuple) -> list|tuple:
        """Truncates the results to the display number."""
        if self.dispn == 0:
            return results
        return results[:self.dispn]

    def convert_results_to_list_table(self, results: dict[str, Any], name:str="") -> list[tuple]:
        """Converts the results to custom text format."""
        # basic_details = {
        #     "input_details": input_details,
        #     "output_details": output_details,
        #     "input_total": input_total,
        #     "output_total": output_total,
        #     "margin": margin,
        #     "amount": amount,
        #     "total_margin": total_margin,
        #     "GP/h": GP_h
        # }
        results["input_details"] = self.transform(results["input_details"], results["amount"])
        results["output_details"] = self.transform(results["output_details"], results["amount"])


        input_total: int = results["input_total"]
        output_total: int = results["output_total"]
        amount:int = results["amount"]
        margin:int = results["margin"]
        total_margin:int = results["total_margin"]
        total_time = results["total_time"]
        gp_h = results["GP/h"]

        # DEBUG: might need to remove the amount* bit
        all_details = [ # pylint: disable=W8301
            (f"Inputs ({name})",None),
            *results["input_details"],
            (f"Total ({name})",None,None,input_total,amount*input_total, None, None),
            self.blank_line,
            (f"Outputs ({name})",None),
            *results["output_details"],
            (f"Total (w/Tax {name})",None,None,output_total,amount*output_total, None, None),
            self.blank_line,
            (f"Profit/Loss ({name})", None,None, margin, total_margin, total_time, gp_h),
        ]

        return all_details

    def combine_list_tables(self, *listtables: list[tuple]) -> list[tuple]:
        """Combines multiple list tables into one."""
        # Each table separated by two blank lines
        final_listtable = []
        num_tables = len(listtables) - 1
        for listtable_num in range(num_tables+1):
            listtable = listtables[listtable_num]
            final_listtable.extend(listtable)
            if listtable_num != num_tables:
                final_listtable.extend([self.blank_line, self.blank_line])

        return final_listtable

    def str_tabulate_listtable(self, listtable: list[tuple], intfmt=",", floatfmt=".2f", numalign="right") -> str:
        """Converts a list table to a string."""
        return tabulate(
            listtable,
            headers=self.item_headers,
            intfmt=intfmt,
            floatfmt=floatfmt,
            numalign=numalign
        )

    def write_specific_recipe_details(self, recipe_s: str, file: IO) -> bool:
        """Writes the details of a specific recipe to a file."""
        recipe_details = self.price_io.specific_recipe_details(recipe_s)
        if recipe_details is None:
            return False

        # Same length
        basic_details = self.convert_results_to_list_table(recipe_details["basic_details"], name="Base")
        pm_details = self.convert_results_to_list_table(recipe_details["pm_details"], f"{self.percent_str} margin")

        all_details = self.combine_list_tables(basic_details, pm_details)

        all_details_str = recipe_s + "\n" + self.str_tabulate_listtable(all_details)


        heading = len(recipe_s) + len("\n")
        length = all_details_str.find("\n", heading) - heading

        print(all_details_str, end="\n\n"+"#"*length+"\n\n", file=file)
        return True

    def write_recipe_lookup(self, *recipe_s_list: str):
        """Writes the details of a specific recipe to a file."""
        self.switch_to_lookup()
        f = self.raw_load(mode="w")

        if f is None:
            return False
        with f:
            self.recipe_lookup_inner(f, *recipe_s_list)
            return True

    def recipe_lookup_inner(self, file:IO, *recipe_s_list: str):
        """Writes the details of a specific recipe to a file."""
        logging.info("Writing recipe lookup/s. to %s", self.filename)
        for recipe_s in recipe_s_list:
            self.write_specific_recipe_details(recipe_s, file)

    def write_all_overview(self, intfmt:str=",", missingval:str="N/A") -> bool:
        """Writes the overview of all recipes to a file."""
        all_recipe_prices = self.price_io.all_recipe_price_overview(
            profiting=self.profiting, show_hidden=self.show_hidden, reverse=self.reverse
        )
        all_recipe_prices = self.truncate_results(all_recipe_prices)

        coins_str = f"CURRENT COINS = {self.price_io.coins:,}"

        all_recipe_prices_str = tabulate(
            all_recipe_prices,
            headers=self.method_headers,
            intfmt=intfmt,
            missingval=missingval
        )
        # Only want recipe name of <topn> recipes
        detailed_recipes  = [recipe_name for (recipe_name, *_) in all_recipe_prices[:self.topn]]
        length = all_recipe_prices_str.find("\n")

        self.switch_to_overview()
        f = self.raw_load(mode="w")
        if f is None:
            return False
        with f:
            print(coins_str + "\n\n" + all_recipe_prices_str, end="\n\n"+"#"*length+"\n\n",
                   file=f
            )
            self.recipe_lookup_inner(f, *detailed_recipes)
            return True

    # Compute total price gain from a recipe and total buying amount
    @staticmethod
    def transform_inner(tup: tuple[Item,tuple[int, int]], afford_amount: float):
        """Calculates the total price of an item, and the amount to buy. Appends to the tuple."""
        (item, (price, recipe_amount)) = tup
        item_name = item.name
        to_buy  = math.ceil(afford_amount*recipe_amount)
        total_price = price*to_buy

        return (item_name, recipe_amount, to_buy, price, total_price)
    @staticmethod
    def transform(pricelist: dict[Item, tuple[int,int]], afford_amount: int):
        """Calculates all details required for table output. Applies to all items in the list."""
        def inner_wrapper(tup):
            return ResultWriter.transform_inner(tup, afford_amount)
        return list(map(inner_wrapper, pricelist.items()))

# dict[str, str|dict[str, float]]
def recipe_hook(dct:  dict, item_search: ItemSearch):
    """Hook for parsing the recipe data from json files."""
    if "name" not in dct:
        return dct
    # Else
    recipe_name = dct["name"]

    dct_inputs = dct["inputs"]
    if not dct_inputs:
        logging.warning("recipeHook: '%s' has no inputs.", recipe_name)
        dct["inputs"] = {}

    dct_outputs = dct["outputs"]
    if not dct_outputs:
        logging.warning("recipeHook: '%s' has no outputs.", recipe_name)
        dct["outputs"] = {}

    # Need to convert the inputs and outputs to ItemList objects
    # {item_name (str): quantity (float)}
    inputs = ItemList()
    outputs = ItemList()


    for item_name, quantity in dct["inputs"].items():
        valid = inputs.add_item_by_str(item_name, quantity, item_search)
        if not valid:
            log_warning("recipeHook: '%s' has invalid input: %s", recipe_name, item_name)
            # Return Recipe object with valid=False
            return Recipe(recipe_name, inputs, outputs, -2)

    for item_name, quantity in dct["outputs"].items():
        valid = outputs.add_item_by_str(item_name, quantity, item_search)
        if not valid:
            log_warning("recipeHook: '%s' has invalid output: %s", recipe_name, item_name)
            return Recipe(recipe_name, inputs, outputs, -2)

    # Time might not exist. -1 if not
    time = dct.get("time", -1)
    return Recipe(recipe_name, inputs, outputs, time)


def load_config(config_fp) -> dict:
    """Loads the config.toml file."""
    config_loader = FileIO(config_fp)
    config = config_loader.raw_load(mode="rb")
    if config is None:
        raise OSError("Failed to load config.toml")
    config = cast(BinaryIO, config) # Known to be BinaryIO
    config = tomli.load(config)
    return config

def compute_weights(coins:int, margin_to_time:float, time:float, gp_h:float):
    """Computes the weights for the optimal sort. Calculates margin to time ratio."""
    # margin_to_time, time, gp_h
    pow10_coins = math.floor(math.log10(coins))
    denom = 10**(pow10_coins-1)

    m_to_tm = (margin_to_time,1/denom)
    m_to_tot_rat = 1/sum(m_to_tm) # Normalise this to between 0 and 1

    return (m_to_tm[0]/m_to_tot_rat, m_to_tm[1]/m_to_tot_rat, time, gp_h)


def main():
    """Main function."""
    # Load config
    config_fp = "config.toml"
    logging.info("Loading: Config from %s", config_fp)
    config = load_config(config_fp)


    # Load all items
    data_fps:ValuesView = config["filepaths"]["data"].values()
    logging.info("Loading items from %s", data_fps)
    data_fps_i:Iterator = iter(data_fps)
    price_data_io = FileIO(next(data_fps_i))
    id_to_name_io = FileIO(next(data_fps_i))
    name_to_id_io = FileIO(next(data_fps_i))


    api_settings = config["API_settings"]
    logging.info("Initialising: API settings for %s", api_settings["url"])
    api = PriceAPI(**api_settings, price_data_IO=price_data_io)

    file_exists = '(File Exists)' if price_data_io.exists()  else '(None)'
    choice = int(input(f"1. API Refresh Data\n2. Load previous Data {file_exists}\n\n>"))

    match choice:
        case 1:
            logging.info("Retrieving prices from API.")
            api_results = api.get_all_prices()
            if api_results is None:
                logging.error("Failed to retrieve prices.")
                return
        case 2:
            logging.info("Loading previous data.")
        case _:
            print("Bad choice "+ str(choice))
            return

    ignore_items = config["filepaths"]["recipes"]["ignore_items"]
    all_items = ItemSearch(None, price_data_io, id_to_name_io, name_to_id_io, ignore=ignore_items)

    # Load all recipes
    recipe_fp = config["filepaths"]["recipes"]["recipe_data"]
    logging.info("Loading: Recipes from %s", recipe_fp)
    recipe_book = RecipeBook(all_items, recipe_fp)


    money_settings = config["profit_settings"]["money"]
    weight_biases = config["profit_settings"]["weights"]
    weights = compute_weights(money_settings["coins"], **weight_biases)
    price_calc = PriceHandle(all_items, recipe_book, **money_settings, weights=weights)


    display_back = config["profit_settings"]["display"]["backend"]
    display_front = config["profit_settings"]["display"]["frontend"]
    display_fps = config["filepaths"]["results"]

    results = ResultWriter(price_calc, **display_back, **display_front, **display_fps)

    results.write_all_overview()
    # results.write_recipe_lookup("Casting Bones to Bananas (F2P)")

if __name__ == "__main__":
    main()
    