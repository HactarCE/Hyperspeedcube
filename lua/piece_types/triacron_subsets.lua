-- Marks all pieces on a triacron-subset puzzle with axes named U, F, R, and L
-- in the expected places.
--
-- The twist `U` and the region `R(1, half_layers)` must generate `U_adj`.
function mark_multilayer_UFRL(puzzle, combined_layers)
  local half_layers = floor(combined_layers/2)
  local U = puzzle.axes.U
  local F = puzzle.axes.F
  local R = puzzle.axes.R
  local L = puzzle.axes.L
  local U_adj = symmetry{puzzle.twists.U}:orbit(R(1, half_layers)):union()
  local UF_adj = R(1, half_layers) | L(1, half_layers)
  local UFR_adj = REGION_NONE
  mark_multilayer(puzzle, combined_layers, U, F, R, U_adj, UF_adj, UFR_adj)
end

-- Marks all pieces on a triacron-subset puzzle
function mark_multilayer(puzzle, combined_layers, U, F, R, U_adj, UF_adj, UFR_adj)
  local half_layers = floor(combined_layers/2)
  if combined_layers % 2 == 1 then
    mark_multilayer_edges(puzzle, half_layers, U, F, U_adj, UF_adj)
  end
  mark_multilayer_corners(puzzle, half_layers, U, F, R, UFR_adj)
end

-- Marks middle edges and centers on a multilayer sphenic (2-acronic) edge block.
function mark_multilayer_edges(puzzle, layers, U, F, U_adj, UF_adj)
  if layers < 1 then
    return
  end

  puzzle:add_piece_type('center', "Center")
  puzzle:add_piece_type('edge', "Edge")

  -- Middle edge + center
  local middle_center_region = U(1) & ~U_adj
  local middle_edge_region = U(1) & F(1) & ~UF_adj
  if layers > 1 then
    puzzle:mark_piece(middle_center_region, 'center/0_0', "Middle center")
    puzzle:mark_piece(middle_edge_region, 'edge/0', "Middle edge")
  else
    puzzle:mark_piece(middle_center_region, 'center')
    puzzle:mark_piece(middle_edge_region, 'edge')
  end

  -- T-centers
  for i = 1, layers-1 do
    local region = U(1) & F(layers-i+1) & ~UF_adj
    puzzle:mark_piece(region, string.fmt2('center/0_%d', "T-center (%d)", i))
  end
end

-- Marks the centers and wings of a multilayer triacronic corner block.
function mark_multilayer_corners(puzzle, layers, U, F, R, UFR_adj)
  if layers < 1 then
    return
  end

  if layers > 1 then
    puzzle:add_piece_type('center', "Center")
    puzzle:add_piece_type('edge', "Edge")
  end

  -- X-centers and oblique centers
  for i = 1, layers-1 do
    for j = 1, layers-1 do
      local region = U(1) & F(layers-i+1) & R(layers-j+1) & ~UFR_adj
      puzzle:mark_piece(region, string.fmt2('center/%d_%d', "Center (%d, %d)", i, j))
    end
  end

  -- Wings
  for i = 1, layers-1 do
    local region = U(1) & F(1) & R(layers-i+1) & ~UFR_adj
    puzzle:mark_piece(region, string.fmt2('edge/%d', "Wing (%d)", i))
  end

  -- Corners
  local region = U(1) & F(1) & R(1) & ~UFR_adj
  puzzle:mark_piece(region, 'corner', "Corner")
end
