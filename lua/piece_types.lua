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
  puzzle:mark_piece(REGION_ALL, 'core', "Core")
end

-- Returns the string "Inner" if `i < threshold`, "Outer" if `i > threshold`, or
-- "Middle" if `i == threshold`.
function inner_outer_prefix(i, threshold)
  if i < threshold then return "Inner" end
  if i > threshold then return "Outer" end
  if i == threshold then return "Middle" end
end

function mark_left_right(puzzle, region, name_pattern, display_pattern, i, j)
  if i < j then
    local sentencecased = display_pattern:sub(1, 1):upper() .. display_pattern:sub(2)
    puzzle:add_piece_type(string.fmt2(name_pattern, sentencecased, i, j))
    puzzle:add_piece_type(string.fmt2(name_pattern .. '/left', "Left " .. display_pattern, i, j))
    puzzle:mark_piece(region, string.fmt2(name_pattern .. '/right', "Right " .. display_pattern, i, j))
  else
    puzzle:mark_piece(region, string.fmt2(name_pattern .. '/left', "Left " .. display_pattern, j, i))
  end
end
