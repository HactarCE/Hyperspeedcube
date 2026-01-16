#!/usr/bin/env python3

from datetime import date
import os
import re
import shutil
import subprocess
import sys
import tempfile

from util import REPO, WORKSPACE_DIR


# Shamelessly stolen from https://stackoverflow.com/a/31499114/4958484
# Modified so as not to keep file attributes, such as modification time
def sed_inplace(filename, pattern, repl):
    ''',
    Perform the pure-Python equivalent of in-place `sed` substitution: e.g.,
    `sed -i -e 's/'${pattern}'/'${repl}' "${filename}"`.
    ''',
    # For efficiency, precompile the passed regular expression.
    pattern_compiled = re.compile(pattern)

    # For portability, NamedTemporaryFile() defaults to mode "w+b" (i.e., binary
    # writing with updating). This is usually a good thing. In this case,
    # however, binary writing imposes non-trivial encoding constraints trivially
    # resolved by switching to text writing. Let's do that.
    with tempfile.NamedTemporaryFile(mode='w', delete=False) as tmp_file:
        with open(os.path.join(WORKSPACE_DIR, filename)) as src_file:
            for line in src_file:
                tmp_file.write(pattern_compiled.sub(repl, line))

    # Overwrite the original file with the munged temporary file.
    shutil.move(tmp_file.name, os.path.join(WORKSPACE_DIR, filename))


version = sys.argv[1] if len(sys.argv) == 2 else input("Enter new version number: v") or exit(1)

# Set package `version`.
sed_inplace('crates/hyperspeedcube/Cargo.toml',
            r'^version = "[^"\n]*"$',
            f'version = "{version}"')

# Set Windows metadata `ProductVersion`.
sed_inplace('crates/hyperspeedcube/Cargo.toml',
            r'^ProductVersion = "[^"]*"$',
            f'ProductVersion = "{version}"')

# Set latest version in changelog.
sed_inplace('CHANGELOG.md',
            r'\[UNRELEASED\]',
            f'[{version}] - {date.today():%Y-%m-%d}')

# Update Cargo.lock
subprocess.run(['cargo', 'update', '--workspace'], check=True)
