#!/bin/sh

# Installation: `ln -s scripts/pre-commit.sh .git/hooks/pre-commit`

# Git runs hooks from the top level of the working tree, but be robust anyway.
cd "$(git rev-parse --show-toplevel)" || exit 1

failure=0

echo "Running cargo +nightly fmt --check..."
if ! cargo +nightly fmt --check; then
    echo "cargo +nightly fmt --check FAILED" >&2
    failure=1
fi

echo "Running typos..."
if ! typos; then
    echo "typos FAILED" >&2
    failure=1
fi

if [ "$failure" -ne 0 ]; then
    echo "pre-commit checks failed; commit aborted." >&2
    exit 1
fi

echo "pre-commit checks passed."
exit 0
