
function build_mixup(plus)
  return function(self)
    local d
    if plus then
      d = sqrt(2)/(4+sqrt(2))
    else
      d = sqrt(2)/(2+sqrt(2))
    end
    local shape = lib.symmetries.bc3.cube()
    lib.utils.cut_ft_shape(self, shape, {INF, d, -d, -INF})

    local U = self.axes.U
    local F = self.axes.F
    local R = self.axes.R

    local function ccw45(fix)
      return rot{fix=fix, angle=pi/4}
    end

    -- unbandage
    --[[
      self:copy_cuts{
        {from: U(2), thru: rot{fix = U, angle = pi/4}},
        {from: REGION_ALL, thru: sym}
      } -- may not be efficient
    ]]
    for t, axU, axR, axF in shape.sym.chiral:orbit(U, R, F) do
      self:slice(
        ccw45(axU):transform(plane{normal = axR, distance = d}),
        { region = axF(1, 2) & axR(1) & axU(2) }
      )
      self:slice(
        (ccw45(axU)*ccw45(axR)):transform(plane{normal = axF, distance = d}),
        { region = axF(1) & axR(1) & axU(2) }
      )
    end

    -- Define twists
    for t, ax, rot in shape.sym.chiral:orbit(U, ccw45(U)) do
      self.twists:add(ax, rot, { gizmo_pole_distance = 1 })
    end

    -- Add piece types
    self:mark_piece(U(1) & R(1) & F(1), 'corner', "Corner")
    self:add_piece_type('mixup', 'Mixup piece')
    self:mark_piece(U(1) & R(2) & F(2), 'mixup/center', "Center")
    self:mark_piece(ccw45(R):transform(U(1) & R(2) & F(2)), 'mixup/edge', "Edge")
    if plus then
      self:mark_piece(ccw45(U).rev:transform(F(1)) & R(1), 't_center', 'T-center')
    end
    self:unify_piece_types(shape.sym.chiral)
  end
end

function build_son_mum(plus)
  return function(self)
    local d = 0.53
    local shape = lib.symmetries.bc3.cube()
    lib.utils.cut_ft_shape(self, shape, {INF, d, -d, -INF})

    local U = self.axes.U
    local F = self.axes.F
    local R = self.axes.R

    local function ccw45(fix)
      return rot{fix=fix, angle=pi/4}
    end

    local edge_region = U(2) & R(1) & F(1)
    local ce_region = ccw45(U):transform(edge_region)

    for t, axU, axR, axF, er in shape.sym.chiral:orbit(U, R, F, edge_region) do
      self:slice(
        ccw45(axU):transform(plane{normal = axR, distance = d}),
        { region = axF(1, 2) & axR(1) & axU(2) }
      )
      if plus then
        self:slice(
          (ccw45(axU)*ccw45(axR)):transform(plane{normal = axF, distance = d}),
          { region = er }
        )
      end
    end

    -- Define twists
    for t, ax, rot in shape.sym.chiral:orbit(U, ccw45(U)) do
      self.twists:add(ax, rot, { gizmo_pole_distance = 1 })
    end

    local center_region = ce_region & rot{fix=R, angle=pi/2}:transform(ce_region)
    local t_center_region = ce_region & ccw45(F):transform(R(2))

    -- Add piece types
    self:mark_piece(U(1) & R(1) & F(1), 'corner', "Corner")
    local mixup_ = ''
    local plus_ = ''
    if plus then
      self:add_piece_type('mixup', 'Mixup piece')
      self:add_piece_type('plus', 'Plus piece')
      mixup_ = 'mixup/'
      plus_ = 'plus/'
      self:mark_piece(ccw45(U).rev:transform(center_region) & edge_region, mixup_ .. 'edge', 'Edge')
      self:mark_piece(ccw45(U).rev:transform(t_center_region) & edge_region, plus_ .. 'wing', 'Wing')
    else
      self:mark_piece(edge_region, 'edge', "Edge")
    end
    self:mark_piece(center_region, mixup_ .. 'center', "Center")
    self:mark_piece(t_center_region, plus_ .. 't_center', "T-center")
    self:mark_piece(U(2) & F(2) & R(1) & ccw45(F):transform(R(2)) & ccw45(U):transform(R(2)), 'x_center', "X-center")
    
    self:unify_piece_types(shape.sym.chiral)
  end
end

puzzles:add{
  id = 'mixup_cube',
  version = '0.1.0',
  name = "Mixup Cube",
  ndim = 3,
  colors = 'cube',
  build = build_mixup(false),

  tags = {
    author = "Milo Jacquet",
    experimental = true,
  },
}

puzzles:add{
  id = 'mixup_plus',
  version = '0.1.0',
  name = "Mixup Cube Plus",
  ndim = 3,
  colors = 'cube',
  build = build_mixup(true),

  tags = {
    author = "Milo Jacquet",
    experimental = true,
  },
}

puzzles:add{
  id = 'son_mum',
  version = '0.1.0',
  name = "Son-Mum Cube",
  ndim = 3,
  colors = 'cube',
  build = build_son_mum(false),

  tags = {
    author = "Milo Jacquet",
    experimental = true,
  },
}

puzzles:add{
  id = 'son_mum_plus',
  version = '0.1.0',
  name = "Son-Mum Cube Plus",
  ndim = 3,
  colors = 'cube',
  build = build_son_mum(true),

  tags = {
    author = "Milo Jacquet",
    experimental = true,
  },
}
