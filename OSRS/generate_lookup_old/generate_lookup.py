"""Generate the name-id lookup tables from item_search.py"""


import json
import tomli

# Some API IDs are not minimum
MANUAL_OVERRIDES: dict[str, str] = {
    "5075": "Bird nest",
}

IDENT_LEVEL = 4

def main():
    # Load config.toml
    with open('config.toml', "rb") as f:
        config = tomli.load(f)
    # [filepaths.data]
    # id_to_name = "id_to_name.json"
    # name_to_id = "name_to_id.json"
    id_to_name_str: str = config['filepaths']['data']['id_to_name']
    name_to_id_str: str = config['filepaths']['data']['name_to_id']
    # Load item_search.json
    with open('lookup_data/item_search.json', "r") as f:
        item_search: dict[str, dict] = json.load(f)
        # JSON list of the form
        # "item_id": { // "id" as str
        #   "id": item_id, // int
        #   "name": "Dwarf remains", // int
        #   "type": "normal", // str
        #   "duplicate": false // bool
        # },


    id_to_name: dict[str,str] = {}
    name_to_id: dict[str,str] = {}
    
    for item_id, item in item_search.items():
        # Skip if type is placeholder
        if item['type'] == 'placeholder':
            continue
        # If duplicate is true, add to dup_names and skip
        if (not item['duplicate']):
            # Check if name is in name_to_id
            name = item['name']
            id_to_name[item_id] = name
            if name in name_to_id.keys():
                # Want value with minimum id
                old_id = name_to_id[name]
                if int(item_id) > int(old_id): # Want minimum id
                    continue
            
            # # Add to both id_to_name and name_to_id
            # id_to_name[item_id] = name
            name_to_id[name] = item_id
        else:
            # Add just to id_to_name
            id_to_name[item_id] = item['name']
    

    # Override id_to_name with MANUAL_OVERRIDES
    for item_id, name in MANUAL_OVERRIDES.items():
        id_to_name[item_id] = name
        name_to_id[name] = item_id

    # Overwrite file contents
    with open(id_to_name_str, "w") as f:
        json.dump(id_to_name, f, indent=IDENT_LEVEL)
    with open(name_to_id_str, "w") as f:
        json.dump(name_to_id, f, indent=IDENT_LEVEL)
    


if __name__ == '__main__':
    main()