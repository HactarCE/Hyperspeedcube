#!/usr/bin/env python3

from datetime import date
import re
import shutil
import subprocess
import sys
import tempfile
import urllib


REPO = "HactarCE/Hyperspeedcube"


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
        with open(filename) as src_file:
            for line in src_file:
                tmp_file.write(pattern_compiled.sub(repl, line))

    # Overwrite the original file with the munged temporary file.
    shutil.move(tmp_file.name, filename)


def do_subcommand(name):
    return len(sys.argv) == 1 or name in sys.argv[1:]


print("Please make sure your working directory is clean before proceeding.")
print()
print("Available subcommands: write, git, release-url")
print()
version = input("Enter new version number: v") or exit(1)
if do_subcommand('git'):
    branch = input("Enter current git branch: ") or exit(1)


if do_subcommand('write'):
    # Set package `version`.
    sed_inplace('Cargo.toml',
                r'^version = "[^"\n]*"$',
                f'version = "{version}"')

    # Set Windows metadata `ProductVersion`.
    sed_inplace('Cargo.toml',
                r'^ProductVersion = "[^"]*"$',
                f'ProductVersion = "{version}"')

    # Set environment variable `HYPERSPEEDCUBE_VERSION` in GitHub Actions workflow.
    sed_inplace('.github/workflows/builds.yml',
                r'HYPERSPEEDCUBE_VERSION: .*',
                f'HYPERSPEEDCUBE_VERSION: {version}')

    # Set latest version in changelog.
    sed_inplace('CHANGELOG.md',
                r'\[UNRELEASED\]',
                f'[{version}] - {date.today():%Y-%m-%d}')

    # Update Cargo.lock
    subprocess.run(['cargo', 'update', '--workspace'], check=True)


if do_subcommand('git'):
    git_commands = [
        f'git add Cargo.toml Cargo.lock .github/workflows CHANGELOG.md',
        f'git commit -m "Version {version}"',
        f'git tag v{version}',
        f'git push origin refs/tags/v{version}',
        f'git checkout stable',
        f'git merge refs/tags/v{version}',
        f'git push',
    ]
    if branch != 'patch':
        git_commands += [
            f'git checkout patch',
            f'git merge refs/tags/v{version}',
            f'git push',
        ]
    if branch != 'main':
        git_commands += [
            f'git checkout main',
            f'git merge refs/tags/v{version}',
            f'git push',
        ]
    if branch != 'dev':
        git_commands += [
            f'git checkout dev',
            f'git merge refs/tags/v{version}',
            f'git push',
        ]
    git_commands += [f'git checkout {branch}']

    print("Updated version numbers in Cargo.toml, GitHub Actions workflow, and changelog. Next steps:")
    print()
    for cmd in git_commands:
        print('  ' + cmd)
    print()
    print("Press enter to run these commands, or ctrl+C to cancel.")
    input()
    for cmd in git_commands:
        subprocess.run(cmd, shell=True, check=True)

    print("Success!")

if do_subcommand('release-url'):
    with open('CHANGELOG.md') as f:
        changelog_body = re.search(
            rf'## \[{version}\].*\n((\n|[^#\n].*\n|###.*\n)*)',
            f.read(),
        ).group(1)
    with open('tmp_release_notes.txt', 'w') as f:
        f.write(
            changelog_body.strip()
            + f"\n\n**[Changelog](https://github.com/{REPO}/blob/main/CHANGELOG.md)**\n"
        )
    print("Click this link to make the release:")
    print(
        f"https://github.com/{REPO}/releases/new"
        f"?tag=v{version}"
        f"&title=v{version}"
        # f"&body=**%5BChangelog%5D(https://github.com/{REPO}/blob/main/CHANGELOG.md)**"
    )
    print("See tmp_release_notes.txt for release body")
