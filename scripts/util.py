import re
import os

WORKSPACE_DIR = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))

CURRENT_VERSION = None

with open(os.path.join(WORKSPACE_DIR, "crates/hyperspeedcube/Cargo.toml")) as f:
    for line in f:
        m = re.search(r'^version = "([^"\n]*)"$', line)
        if m:
            CURRENT_VERSION = m.group(1)
            break
    else:
        print("could not determine version")
        exit(1)
