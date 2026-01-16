#!/usr/bin/env python3

import os
import subprocess

from util import REPO, WORKSPACE_DIR

os.chdir(WORKSPACE_DIR)

DOWNLOAD_DIR = 'release_binaries'

if os.path.exists(DOWNLOAD_DIR):
    print("Deleting existing downloads ...")
    for f in os.listdir(DOWNLOAD_DIR):
        os.remove(os.path.join(DOWNLOAD_DIR, f))
    os.rmdir(DOWNLOAD_DIR)

os.makedirs(DOWNLOAD_DIR)
os.chdir(DOWNLOAD_DIR)

for os_name in ['linux', 'win64', 'macos']:
    print(f"Downloading binary for {os_name} ...")
    subprocess.run(f'gh run download --repo {REPO} --name hyperspeedcube_{os_name}', shell=True, check=True)

print("Rezipping windows build ...")
subprocess.run('zip hyperspeedcube_windows.zip hyperspeedcube.exe', shell=True)
print("Deleting windows EXE ...")
os.remove('hyperspeedcube.exe')

print(f"Downloaded binaries to {os.path.realpath(os.getcwd())}")
