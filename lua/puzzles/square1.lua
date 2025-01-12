
function build_sq1(n)
  return function(self)
    local sym = cd{12, 2}
    local shape_sym = symmetry{sym:thru(1,2,1,2,1,2), sym:thru(3)}
    local width = cos(pi/12)

    self:carve(shape_sym:orbit(sym.oxo.unit * width))
    self:carve(shape_sym:orbit(sym.oox.unit))

    --self.axes:add(shape_sym:orbit(sym.oox.unit), {INF, width - sin(pi/12)})
    U = self.axes:add(sym.oox.unit, {INF, 1 - width + sin(pi/12)})
    D = self.axes:add(sym:thru(3):transform(sym.oox.unit), {INF, 1 - width + sin(pi/12)})
    R = self.axes:add(sym:thru(2,1,2,1,2,1,2):transform(sym.xoo), {INF, 0, name = "R"})

    local seed_rot
    if n == 0 then
      seed_rot = sym:thru(1,2,1,2,1,2)
    else
      seed_rot = sym:thru(1,2)
    end
    for t, ax, rot in shape_sym:orbit(U, seed_rot) do
      self.twists:add(ax, rot) --, { gizmo_pole_distance = 1 })
    end
    self.twists:add(R, rot{fix=R, angle=pi}) --, { gizmo_pole_distance = 1 })

    local cuts
    if n == 0 then
      cuts = {3}
    elseif n == 1 then
      cuts = {1, 3, 4}
    elseif n == 2 then
      cuts = {1, 2, 3, 4, 5}
    end
    for _, i in ipairs(cuts) do
      self:slice((sym:thru(1,2)^i):transform(plane{normal = R.vector, distance = 0}), {region=U(1)|D(1)})
    end

    -- Piece types
    if n == 0 then
      self:mark_piece(U(1) & R(1) & sym:thru(1,2,1,2,1,2):transform(~R(1)), 'corner', "Corner")
    elseif n == 1 then
      self:mark_piece(U(1) & R(1) & sym:thru(1,2):transform(~R(1)), 'edge', "Edge")
      self:mark_piece(U(1) & R(1) & sym:thru(2,1,2,1):transform(~R(1)), 'corner', "Corner")
    elseif n == 2 then
      self:add_piece_type('cheese', "Cheese")
      self:mark_piece(U(1) & R(1) & sym:thru(1,2):transform(~R(1)), 'cheese/edge', "Edge")
      self:add_piece_type('cheese/wing', "Wing")
      wing_region = U(1) & R(1) & sym:thru(2,1):transform(~R(1))
      self:mark_piece(wing_region, 'cheese/wing/left', "Wing (left)")
      self:mark_piece(sym:thru(1):transform(wing_region), 'cheese/wing/right', "Wing (right)")
    end  
    self:unify_piece_types(shape_sym)
    
    self:mark_piece(~U(1) & ~D(1), 'center', "Slash center")
    print("warning intentional, matching 2 pieces")

  end
end


puzzles:add{
  id = 'square_1',
  version = '0.1.0',
  name = "Square-1",
  ndim = 3,
  -- colors = 'square_1',
  build = build_sq1(1),

  tags = {
    author = "Milo Jacquet",
    experimental = true,
  },
}

puzzles:add{
  id = 'square_2',
  version = '0.1.0',
  name = "Square-2",
  ndim = 3,
  -- colors = 'square_1',
  build = build_sq1(2),

  tags = {
    author = "Milo Jacquet",
    experimental = true,
  },
}

puzzles:add{
  id = 'square_0',
  version = '0.1.0',
  name = "Square-0",
  ndim = 3,
  -- colors = 'square_1',
  build = build_sq1(0),

  tags = {
    author = "Milo Jacquet",
    experimental = true,
  },
}
