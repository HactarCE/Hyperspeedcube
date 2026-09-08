# hyperpuzzle

Global puzzle catalog for Hyperspeedcube

## Architecture

### Catalog

- A **`Catalog`** contains all `Generator`s for catalog objects. There is typically one global catalog, acquired via `hyperpuzzle::catalog()`.
- A **`CatalogObject`** is an object that can be generated lazily, such as a `Puzzle`, `TwistSystem`, `ColorSystem`, or even `PuzzleListMetadata`. It is identified by a `CatalogId`.
- A **`Generator`** is a parametrized catalog object. It is identified by a the `generator_name` and `@version` components of a `CatalogId`.
- A **`CatalodId`** uniquely identifies a `CatalogObject`.
  - A general catalog ID takes the form `generator_name@version.subsets(parameters)`
    - `generator_name` consists of lowercase ASCII alphanumeric + hyphens + underscores.
    - `version` is a nonnegative integer. `0` indicates an unstable object; `1` or higher indicates a stable one. `@version` may be omitted to select the latest version.
    - `subsets` is a dot-separated list of twist subsets, which are defined by the generator. When empty, the `.` is omitted.
    - `parameters` is a comma-separated list of `CatalogIdValue`. When empty, the parentheses are omitted.
  - Note that integers (e.g., `-15`) are valid `CatalogId`s, even though they do not correspond to any catalog object.
- A **`CatalogIdValue`** may be the wildcard `*`, the error value `!`, a catalog ID (which may itself contain parameters, etc.), or a list of parameters.
  - Most catalog ID values are integers, which are represented as `CatalogId`s.

### Backends and engines

- A **backend** is a crate that supplies `Generators` to a `Catalog`. It may do this by directly adding them to the catalog, or via Hyperpuzzlescript by registering one or more **engine**s.
  - At the time of writing, `hypuz_impl_euclid` is the only backend.
  - Planned backends include:
    - `hypuz_impl_flat` (Flat Hypercube-style)
    - `hypuz_impl_cga2d` (2D CGA)
    - `hypuz_impl_cbh3` (Conformal Boundary of H3)
- An **engine** is a type that is registered with the Hyperpuzzlescript runtime for puzzle and twist system generator definitions.
  - The engine is called for every `add_puzzle()` and `add_twist_system()` HPS call.

### Components

- A `Component` is additional optional functionality attached to a `CatalogObject`. These are used for rendering and interaction.
- A `ComponentList` stores up to one `Component` per type.
