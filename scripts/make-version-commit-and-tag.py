#!/usr/bin/env python3

import os
import subprocess

from util import CURRENT_VERSION, WORKSPACE_DIR

os.chdir(WORKSPACE_DIR)

git_commands = [
    f'git add crates/hyperspeedcube/Cargo.toml Cargo.lock',
    f'git commit -m "Version {CURRENT_VERSION}"',
    f'git push',
    f"git tag v{CURRENT_VERSION}",
    f"git push origin refs/tags/v{CURRENT_VERSION}"
]

print("Press enter to run each command, or ctrl-C to abort")
for cmd in git_commands:
    print(cmd)
    input()
    subprocess.run(cmd, shell=True, check=True)
    print()
