#!/usr/bin/env python3

import os
import re

from util import CURRENT_VERSION, WORKSPACE_DIR

REPO = "HactarCE/Hyperspeedcube"


changelog_body = ''
with open(os.path.join(WORKSPACE_DIR, 'CHANGELOG.md')) as f:
    m = re.search(
        rf'## \[{CURRENT_VERSION}\].*\n((\n|[^#\n].*\n|###.*\n)*)',
        f.read(),
    )
    if m:
        changelog_body = m.group(1)
with open(os.path.join(WORKSPACE_DIR, 'tmp_release_notes.txt'), 'w') as f:
    f.write(
        changelog_body.strip()
        + f"\n\n**[Changelog](https://github.com/{REPO}/blob/main/CHANGELOG.md)**\n"
    )
print("Click this link to make the release:")
print(
    f"https://github.com/{REPO}/releases/new"
    f"?tag=v{CURRENT_VERSION}"
    f"&title=v{CURRENT_VERSION}"
    # f"&body=**%5BChangelog%5D(https://github.com/{REPO}/blob/main/CHANGELOG.md)**"
)
print("See tmp_release_notes.txt for release body")
