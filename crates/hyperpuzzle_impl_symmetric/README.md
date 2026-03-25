# hyperpuzzle_impl_symmetric

General symmetric puzzle backend for Hyperspeedcube

## To-do

- [x] proper axis names
- [x] 3D twist gizmos
- [ ] 4D twist gizmos
- [x] 3D twist notation
- [x] sticker shrink
- [ ] color systems
- [ ] better error messages (HPS logs)
- [x] fix axis names
- [x] merge axes and named points, or otherwise ensure that the lowercase letters remain in sync
  - examples of symmetric puzzles that require non-axis named points? could maybe be resolved by adding extra axes that are not allowed to turn?
- [x] do expensive calculations in `build` function, not when generating spec
- [ ] Hyperpuzzlescript API
