# HSC2 Puzzle Development Guide

This guide is a work-in-progress. Currently it is a dumping ground for half-baked reference materials that will eventually make their way into more-polished documentation.

## Tags checklist

All tags are boolean unless otherwise specified

- Construction: `solid`, `tiling`, or `soup`
- Dimension: `ndim` (int) or `ndim_generic`
- Algebraic classes: `bandaged`, `doctrinaire`, or `jumbling`
- `author` (string list)
- `inventor` (string list)
- `leaderboard`
- `canonical_id` (string)
- `stable` (version of HSC that has this puzzle stable)
- `big`

## For generators

- `twists` (ID string) (if always same twist system)
- `colors` (ID string) (if always same color system)

## Definitions

- **Doctrinaire** — all moves are always available (never blocked)
- **Jumbling** — cannot be unbandaged to a doctrinaire puzzle with finitely many pieces
- **Bandaged** — neither doctrinaire nor jumbling; moves are sometimes blocked, but the puzzle can be unbandaged to a doctrinaire puzzle with finitely many pieces

Puzzles with finitely many pieces always fall into exactly one of those categories.
