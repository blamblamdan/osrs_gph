### Uses mapping.json from api instead of item_search.json
### prices.runescape.wiki/api/v1/osrs/mapping

"""Generate the name-id lookup tables from item_search.py"""


import json
from locale import getpreferredencoding
from typing import Any
import tomli
# Some API IDs are not minimum
MANUAL_OVERRIDES: dict[str, str] = {
    "5075": "Bird nest",
}
IDENT_LEVEL = 4
ENCODING = getpreferredencoding() # or utf-8

def main():
    """Main."""
    # Load config.toml
    with open('config.toml', "rb") as f:
        config = tomli.load(f)
    # [filepaths.data]
    # id_to_name = "id_to_name.json"
    # name_to_id = "name_to_id.json"
    id_to_name_str: str = config['filepaths']['data']['id_to_name']
    name_to_id_str: str = config['filepaths']['data']['name_to_id']
    # Load item_search.json
    with open('lookup_data/mapping.json', "r",encoding=ENCODING) as f:
        mapping: list[dict[str, Any]] = json.load(f)
        # JSON list of the form
        # {
        #     "highalch": uint,
        #     "members": bool,
        #     "name": String,
        #     "examine": String,
        #     "id": uint,
        #     "value": Integer,
        #     "icon": String,
        #     "lowalch": uint
        # },


    id_to_name: dict[str,str] = {}
    name_to_id: dict[str,str] = {}

    for item in mapping:
        item_id = str(item["id"])
        item_name = item["name"]
        
        id_to_name[item_id] = item_name
        name_to_id[item_name] = item_id

    # Override id_to_name with MANUAL_OVERRIDES
    for item_id, name in MANUAL_OVERRIDES.items(): # pylint: disable=W8202
        id_to_name[item_id] = name
        name_to_id[name] = item_id

    # Overwrite file contents
    with open(id_to_name_str, "w", encoding=ENCODING) as f:
        json.dump(id_to_name, f, indent=IDENT_LEVEL)
    with open(name_to_id_str, "w",encoding=ENCODING) as f:
        json.dump(name_to_id, f, indent=IDENT_LEVEL)


if __name__ == '__main__':
    main()
