#!/bin/bash

if [ $# -eq 0 ]; then
    echo "Provide a version number as argument; e.g.,"
    echo
    echo "$0 1.0.0"
    exit 1
fi

# Set package `version`.
sed -i -r "s/^version = \"[^\"]*\"/version = \"$1\"/" Cargo.toml

# Set Windows metadata `ProductVersion`.
sed -i -r "s/^ProductVersion = \"[^\"]*\"/ProductVersion = \"$1\"/" Cargo.toml

# Set environment variable `HYPERSPEEDCUBE_VERSION` in GitHub Actions workflow
sed -i -r "s/HYPERSPEEDCUBE_VERSION: [^\\n]*/HYPERSPEEDCUBE_VERSION: $1/" .github/workflows/*.yml
