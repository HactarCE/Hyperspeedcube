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

  puzzle:add_piece_type{ name = 'center', display = "Center" }
  puzzle:add_piece_type{ name = 'edge', display = "Edge" }

  -- Middle edge + center
  local middle_center_region = U(1) & ~U_adj
  local middle_edge_region = U(1) & F(1) & ~UF_adj
  if layers > 1 then
    puzzle:mark_piece{
      region = middle_center_region,
      name = 'center/0_0', display = "Middle center",
    }
    puzzle:mark_piece{
      region = middle_edge_region,
      name = 'edge/0', display = "Middle edge",
    }
  else
    puzzle:mark_piece{ region = middle_center_region, name = 'center' }
    puzzle:mark_piece{ region = middle_edge_region, name = 'edge' }
  end

  -- T-centers
  for i = 1, layers-1 do
    local region = U(1) & F(layers-i+1) & ~UF_adj
    local name, display = string.fmt2('center/0_%d', "T-center (%d)", i)
    puzzle:mark_piece{ region = region, name = name, display = display }
  end
end

-- Marks the centers and wings of a multilayer triacronic corner block.
function mark_multilayer_corners(puzzle, layers, U, F, R, UFR_adj)
  if layers < 1 then
    return
  end

  if layers > 1 then
    puzzle:add_piece_type{ name = 'center', display = "Center" }
    puzzle:add_piece_type{ name = 'edge', display = "Edge" }
  end

  -- X-centers and oblique centers
  for i = 1, layers-1 do
    for j = 1, layers-1 do
      local region = U(1) & F(layers-i+1) & R(layers-j+1) & ~UFR_adj
      local name, display = string.fmt2('center/%d_%d', "Center (%d, %d)", i, j)
      puzzle:mark_piece{ region = region, name = name, display = display }
    end
  end

  -- Wings
  for i = 1, layers-1 do
    local name, display = string.fmt2('edge/%d', "Wing (%d)", i)
    puzzle:mark_piece{
      region = U(1) & F(1) & R(layers-i+1) & ~UFR_adj,
      name = name, display = display,
    }
  end

  -- Corners
  puzzle:mark_piece{
    region = U(1) & F(1) & R(1) & ~UFR_adj,
    name = 'corner', display = "Corner",
  }
end
