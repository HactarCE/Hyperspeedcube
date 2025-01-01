-- Marks all pieces on a triacron-subset puzzle with axes named U, F, R, and L
-- in the expected places.
--
-- The twists `UR` and `UF` and the region `R(1, half_layers)` must generate
-- `U_adj`. The twist `UR` and the region `R(1, half_layers)` must generate
-- `UF_adj`.
function mark_multilayer_UFRLIO(puzzle, combined_layers)
  local half_layers = floor(combined_layers/2)
  local U = puzzle.axes.U
  local F = puzzle.axes.F
  local R = puzzle.axes.R
  local L = puzzle.axes.L
  local I = puzzle.axes.I
  local O = puzzle.axes.O
  local U_adj = symmetry{puzzle.twists.UR, puzzle.twists.UF}:orbit(R(1, half_layers)):union()
  local UF_adj = symmetry{puzzle.twists.UF}:orbit(R(1, half_layers)):union()
  local UFR_adj = I(1, half_layers) | O(1, half_layers)
  local UFRI_adj = REGION_NONE
  mark_multilayer(puzzle, combined_layers, U, F, R, I, U_adj, UF_adj, UFR_adj, UFRI_adj)
end

-- Marks all pieces on a triacron-subset puzzle
function mark_multilayer(puzzle, combined_layers, U, F, R, I, U_adj, UF_adj, UFR_adj, UFRI_adj)
  local half_layers = floor(combined_layers/2)
  if combined_layers % 2 == 1 then
    mark_multilayer_ridges(puzzle, half_layers, U, F, U_adj, UF_adj)
    mark_multilayer_edges(puzzle, half_layers, U, F, R, UFR_adj)
  end
  mark_multilayer_corners(puzzle, half_layers, U, F, R, I, UFRI_adj)
end

-- Marks middle edges and centers on a multilayer sphenic (2-acronic) edge block.
function mark_multilayer_ridges(puzzle, layers, U, F, U_adj, UF_adj)
  if layers < 1 then
    return
  end

  puzzle:add_piece_type('center', "Center")
  puzzle:add_piece_type('ridge', "Ridge")

  -- Middle ridge + center
  local middle_center_region = U(1) & ~U_adj
  local middle_ridge_region = U(1) & F(1) & ~UF_adj
  if layers > 1 then
    puzzle:mark_piece(middle_center_region, 'center/middle', "Middle center")
    puzzle:mark_piece(middle_ridge_region, 'ridge/middle', "Middle ridge")
  else
    puzzle:mark_piece(middle_center_region, 'center')
    puzzle:mark_piece(middle_ridge_region, 'ridge')
  end

  -- T-centers
  for i = 1, layers-1 do
    local region = U(1) & F(layers-i+1) & ~UF_adj
    puzzle:mark_piece(region, string.fmt2('center/0_0_%d', "T-center (%d)", i))
  end
end

-- Marks the centers and T-ridges of a multilayer triacronic edge block.
function mark_multilayer_edges(puzzle, layers, U, F, R, UFR_adj)
  if layers < 1 then
    return
  end

  puzzle:add_piece_type('center', "Center")
  puzzle:add_piece_type('ridge', "Ridge")
  puzzle:add_piece_type('edge', "Edge")

  -- X-centers and oblique centers
  for i = 1, layers-1 do
    for j = i, layers-1 do
      local region = U(1) & F(layers-i+1) & R(layers-j+1) & ~UFR_adj
      puzzle:mark_piece(region, string.fmt2('center/0_%d_%d', "Center (0, %d, %d)", i, j))
    end
  end

  -- T-ridges
  for i = 1, layers-1 do
    local region = U(1) & F(1) & R(layers-i+1) & ~UFR_adj
    puzzle:mark_piece(region, string.fmt2('ridge/0_%d', "T-ridge (%d)", i))
  end

  -- Edges
  local region = U(1) & R(1) & F(1) & ~UFR_adj
  if layers > 1 then
    puzzle:mark_piece(region, 'edge/middle', "Middle edge")
  else
    puzzle:mark_piece(region, 'edge')
  end
end

-- Marks the centers and wings of a multilayer triacronic corner block.
function mark_multilayer_corners(puzzle, layers, U, F, R, I, UFRI_adj)
  if layers < 1 then
    return
  end

  if layers > 1 then
    puzzle:add_piece_type('center', "Center")
    puzzle:add_piece_type('ridge', "Ridge")
    puzzle:add_piece_type('edge', "Edge")
  end

  -- X-centers, Y-centers, and oblique centers
  for i = 1, layers-1 do
    for j = i, layers-1 do
      for k = i, layers-1 do
        local is_chiral = i ~= j and j ~= k and i ~= k
        if not is_chiral and j > k then goto continue end
        local region = U(1) & R(layers-i+1) & F(layers-j+1) & I(layers-k+1) & ~UFRI_adj
        puzzle:mark_piece(region, string.fmt2('center/%d_%d_%d', "Center (%d, %d, %d)", i, j, k))
        ::continue::
      end
    end
  end

  -- X-ridges and oblique ridges
  for i = 1, layers-1 do
    for j = i, layers-1 do
      local region = U(1) & F(1) & R(layers-i+1) & I(layers-j+1) & ~UFRI_adj
      puzzle:mark_piece(region, string.fmt2('ridge/%d_%d', "Ridge (%d, %d)", i, j))
    end
  end

  -- Wings
  for i = 1, layers-1 do
    local region = U(1) & F(1) & R(1) & I(layers-i+1) & ~UFRI_adj
    puzzle:mark_piece(region, string.fmt2('edge/%d', "Wing (%d)", i))
  end

  -- Corners
  local region = U(1) & F(1) & R(1) & I(1) & ~UFRI_adj
  puzzle:mark_piece(region, 'corner', "Corner")
end
