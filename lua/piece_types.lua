-- Parameter conventions:
-- - `layers` is the number of shallow-cut subdivions. It is 1 for a 2x2x2 or
--   3x3x3 Rubik's cube, 2 for a 4x4x4 or 5x5x5, etc.
-- - `X_adj` is a region that contains all overlap between `X` and other grips.
--   For example, `UF_adj` must contain all pieces in `U'*' | F'*'` that have grips other
--   than `U` or `F`. It may contain any pieces outside of `U'*' | F'*'`.
--
-- These functions do not call `unify_piece_types()`, so you'll still have to do
-- that in the puzzle definition.

function mark_everything_core(puzzle)
  puzzle:mark_piece{
    region = REGION_ALL,
    name = 'core',
    display = "Core",
  }
end
