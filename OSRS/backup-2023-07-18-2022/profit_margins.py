import requests
import json
# import attrs
from pathlib import Path
from tabulate import tabulate
URL = "https://prices.runescape.wiki/api/v1/osrs"
#URL = "https://eros.atlasservers.net"
HEADERS = {
    'User-Agent': 'profit_margins - @blamblamdan'
}

# Loaded data
prices = {}

# Lookup tables
id_to_name = {}
name_to_id = {}

# Recipes for money making
recipes = {}


# @attrs.define
# class Item:
#     item_id: int
#     cost: int

# @attrs.define
# class ItemList:
#     items: list[tuple[Item, int]] = attrs.Factory(list[tuple[int,int]]) 

# @attrs.define
# class Recipe:
#     inputs: ItemList # [id, cost]
#     outputs: ItemList

def loadData():
    with open("data.json", "r") as file:
        global prices, time
        json_file = json.load(file)
        prices = json_file["data"]

def loadLookups():
    with open("id_to_name.json", "r") as file:
        global id_to_name
        id_to_name = json.load(file)

    with open("name_to_id.json", "r") as file2:
        global name_to_id
        name_to_id = json.load(file2)
    
def loadAll():
    print("\nLoading Data")
    loadData()
    print("Loading Lookup Tables")
    loadLookups()
    print("Loading Recipes")
    getAllRecipes()
    print("Finished Loading.\n")

def writeData(response: str, filename="data.json"):
    with open(filename, "w+") as file:
        json.dump(response, file, indent = 4)


def API(endpoint, headers=HEADERS ,url=URL):
    return requests.get(url+endpoint, headers=headers)

def handleRequest(endpoint, callback, *args, **kwargs):
    with API(endpoint) as r:
        print("Request sent")
    try:
        r.raise_for_status()
    except :
        print("Bad response")
        print(r.text)
        exit()
    else:
        callback(r.json())

def getAllPrices(timespan: str):
    ts_map: set[str] = {
        "latest",
        "5m",
        "1h"
    }
    if not timespan in ts_map:
        print(f"Timespan {timespan} not supported.")
        return
    
    if not timespan.startswith("/"):
        timespan = "/"+timespan

    handleRequest(timespan, writeData)

def getStoredPriceData(itemId: str):
    return prices[itemId]

# def parseRecipe(dct):
#     # <display name: str>:
#     # inputs: {}
#     # outputs: {}
#     # <item name: str>: <amount: int>

#     # Check if value is an integer --> replace key with id_to_name
#     if isinstance(dct[next(iter(dct))], int):
#         dct = {name_to_id[name]:amount for (name,amount) in dct.items()}

#     return dct
    
def buyPrice(itemID: str):
    return prices[itemID]["high"]
def sellPrice(itemID: str):
    return prices[itemID]["low"]

def getPrice(itemID: str, buy: bool):
    """buying (bool): true = buyPrice, false = sellPrice"""
    buying = buyPrice(itemID)
    selling = sellPrice(itemID)

    buy_null = buying is None
    sell_null = selling is None

    assert not (buy_null and sell_null), f"No price information for {id_to_name[itemID]}"
    

    if (buy and buy_null) or (not buy and not sell_null):
        return selling
    else:
        return buying

def priceDetails(items, priceType) -> list[tuple]:
    # (price, amount)
    return [(name, items[name], priceType(name_to_id[name])) for name in items]

def totalPrice(items, priceType) -> int:
    """
    items (iterable)
    priceType (function): buyPrice or sellPrice
    """
    # sum{<itemID>:<amount>}
    columns = [i[1:] for i in priceDetails(items,priceType)]
    return sum(map(lambda item: item[0]*item[1], columns))

def recipeMarginDetails(recipeName) -> list[int]:
    recipe = recipes[recipeName]
    cost = totalPrice(recipe["inputs"], lambda i: getPrice(i, True))
    revenue = totalPrice(recipe["outputs"], lambda j: getPrice(j, False))
    return [cost, revenue, revenue - cost]

def recipeMargin(recipeName) -> int:
    return recipeMarginDetails(recipeName)[-1]

def getAllRecipes():
    with open("recipes.json","r") as file:
        global recipes
        json_file = json.load(file)#, object_hook=parseRecipe)
        recipes = json_file
        recipes.pop("Template")

def recipeDetails(recipeName):
    recipe = recipes[recipeName]
    (input_total, output_total, margin) = recipeMarginDetails(recipeName)
    input_details = priceDetails(recipe["inputs"], lambda i: getPrice(i, True))
    output_details = priceDetails(recipe["outputs"], lambda i: getPrice(i, False))

    item_headers = ("Item", "Amount", "Price")

    blank_line = (None,None,None)
    all_details = [("Inputs",None,None),*input_details,("Total",None,input_total),blank_line,("Outputs",None,None),*output_details, ("Total", None, output_total), blank_line, ("Profit/Loss", None, margin)]

    print(recipeName)
    print(tabulate(all_details, headers=item_headers))



def getAllMargins():
    profitMargins = [(recipe,recipeMargin(recipe)) for recipe in recipes]

    profitMargins = sorted(profitMargins, key=lambda i: i[1], reverse=True)
    return profitMargins
    


def displayAllMargins():
    profitMargins = getAllMargins()
    
    table = tabulate(profitMargins, headers=("Method", "Loss/Gain"))
    print(table, end="\n\n")

def displayMargins(n=5):
    profitMargins = getAllMargins()
    if n == 0:
        pass
    else:
        profitMargins = profitMargins[:n]
    
    
    
    table = tabulate(profitMargins, headers=("Method", "Loss/Gain"))
    print(table, end="\n\n")
    

def main():
    print("EXECUTING MAIN")
    choice = int(input(f"1. API Refresh Data\n2. Load previous Data {'(Exists)' if Path('data.json').is_file()  else '(None)'}\n\n>"))
    
    match choice:
        case 1:
            getAllPrices("latest")
        case 2:
            pass
        case _:
            print("Bad choice "+ str(choice))
    loadAll()
    
    displayMargins(0)
    #recipeDetails("Irit Potion")
    



if __name__ == "__main__":
    #main()
    
    
    
    
    ## UPDATING
    # loadAll()
    # exchange_items_ids = set(prices.keys())
    # with open("item_search.json", "r") as f:
    #     all_items = json.load(f)
    # all_items_ids = set(all_items.keys())
    
    # non_dupe_contained = {k:v for (k,v) in all_items.items() if k in exchange_items_ids}

    # id_to_name = {item: values["name"] for (item, values) in non_dupe_contained.items()}
    # name_to_id = {v:k for (k,v) in id_to_name.items()}

    # writeData(id_to_name, "id_to_name.json")
    # writeData(name_to_id, "name_to_id.json")



    ## CHECKING
    # loadData()
    # loadLookups()

    # price_keys = set(prices.keys())

    # with open("item_search.json", "r") as f:
    #     all_items = json.load(f)
    # all_items_ids = set(all_items.keys())

    # id_keys = set(id_to_name.keys())

    # # 26247 is pumpkin pie (unavailable)
    # diff_list = sorted(list(price_keys.difference(all_items_ids)),key=int)
    # diff_list.remove("26247")
    # print(diff_list)
    # print(sorted(list(id_keys.symmetric_difference(price_keys)), key=int))
    