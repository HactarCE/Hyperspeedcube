# Releasing

To publish a new version:

1. Run `python3 scripts/set-version.py`
2. Run `python3 scripts/make-version-commit-and-tag.py`
3. Wait 10-15 minutes for the [GitHub Actions "build" workflow](https://github.com/HactarCE/Hyperspeedcube/actions/workflows/builds.yml) to finish
4. Run `python3 scripts/prep-binaries.py`
5. Run `python3 scripts/make-release-url.py` and click the link to make a release
