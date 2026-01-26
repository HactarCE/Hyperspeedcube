#!/usr/bin/env python3

import os
import re
import sys
import urllib.request

MDI_JS_URL = 'https://raw.githubusercontent.com/Templarian/MaterialDesign-JS/refs/heads/master/mdi.js'
TEMP_FILE_PATH = os.path.join(os.path.dirname(__file__), '__pycache__/mdi.js')
SVG_TEMPLATE = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="white" d="{}" /></svg>\n'

if len(sys.argv) <= 1:
    print(f"usage: python3 {sys.argv[0]} [--refresh] [icon-name]...")
    sys.exit()

if '--refresh' in sys.argv[1:] or not os.path.isfile(TEMP_FILE_PATH):
    print("Downloading mdi.js ...")
    if not os.path.isdir(os.path.dirname(TEMP_FILE_PATH)):
        os.makedirs(os.path.dirname(TEMP_FILE_PATH))
    urllib.request.urlretrieve(MDI_JS_URL, TEMP_FILE_PATH)

ICON_DEF_REGEX = re.compile(r'export var mdi([a-zA-Z0-9]+) = "([^"]*)";')

print("Reading icon data ...")
with open(TEMP_FILE_PATH) as f:
    icon_data = dict(m.groups() for m in map(ICON_DEF_REGEX.match, f) if m)

def kebab_to_pascal_case(s: str) -> str:
    return s[0].upper() + re.sub(r'-(.)', lambda m: str.upper(m.group(1)), s[1:])

for arg in sys.argv[1:]:
    if arg == '--refresh':
        continue
    k = kebab_to_pascal_case(arg)
    print(f"Creating {arg}.svg ...")
    if k in icon_data:
        with open(f'{arg}.svg', 'w') as out:
            out.write(SVG_TEMPLATE.format(icon_data[k]))
    else:
        print(f"No icon with name '{arg}'")

print("Done!")
