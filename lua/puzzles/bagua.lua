-- issues:
-- - bandaging on D+ R2 U- R- F*
-- - how to name 45-degree turns?
-- - multistage scrambling
-- - center orientation hints?
puzzles:add{
  id = 'bagua_cube',
  version = '0.1.0',
  name = "Bagua Cube",
  ndim = 3,
  colors = 'cube',
  build = function(self)
    local d = 1/2

    local shape = lib.symmetries.bc3.cube()
    lib.utils.cut_ft_shape(self, shape, {INF, d})

    lib.utils.unpack_named(_ENV, self.axes)

    local function ccw45(fix)
      return rot{fix=fix, angle=pi/4}
    end
    local function diag_cut(fix, a, _b)
      return ccw45(fix):transform(plane(a.vector.unit * d))
    end

    local UF_edge_region = F(1) & ~L(1) & ~R(1)
    for t, diag_cut, region_to_cut in shape.sym:orbit(diag_cut(F, R, U), UF_edge_region) do
      self:slice(diag_cut, { region = region_to_cut })
    end

    -- Define twists
    for t, ax, rot in shape.sym.chiral:orbit(F, ccw45(F).rev) do
      self.twists:add(ax, rot, { gizmo_pole_distance = 1 })
    end

    do -- Mark piece types
      local center_region = F(1) & ~symmetry{ccw45(F, R, U)}:orbit(U(1)):union()
      self:mark_piece(center_region, 'center', "Center")

      local inner_triangle_region = F(1) & ~diag_cut(F, R, U).region & ~R(1) & ~U(1)
      self:mark_piece(inner_triangle_region, 'triangle', "Triangle")
      local outer_triangle_region = F(1) & U(1) & diag_cut(F, R, U).region & diag_cut(F, U, L).region
      self:mark_piece(outer_triangle_region, 'triangle')

      local edge_region = ~diag_cut(F, R, U).region & ~diag_cut(F, U, L).region & ~diag_cut(U, F, R).region & ~diag_cut(U, L, F).region
      self:mark_piece(edge_region, 'edge', "Edge")

      self:add_piece_type('oblique', "Oblique")
      local oblique_region = F(1) & U(1) & ~R(1) & ~diag_cut(F, R, U).region & diag_cut(F, U, L).region & diag_cut(F, U, L).region & ~diag_cut(U, L, F).region
      self:mark_piece(oblique_region, 'oblique/left', "Left Oblique")
      self:mark_piece(refl('x'):transform(oblique_region), 'oblique/right', "Right Oblique")

      local wing_region = F(1) & U(1) & ~R(1) & diag_cut(U, L, F).region & diag_cut(F, U, L).region
      self:mark_piece(wing_region, 'wing', "Wing")

      self:mark_piece(U(1) & R(1) & F(1), 'corner', "Corner")

      self:unify_piece_types(shape.sym.chiral) -- chiral because left vs. right obliques
    end
  end,

  tags = {
    author = "Andrew Farkas",
    experimental = true,
  },
}

puzzles:add{
  id = 'baguaminx',
  version = '0.1.0',
  name = "Baguaminx",
  ndim = 3,
  colors = 'dodecahedron',
  build = function(self)
    local d = 1/5 * (-5 + 4 * sqrt(5))

    local shape = lib.symmetries.h3.dodecahedron()
    lib.utils.cut_ft_shape(self, shape, {INF, d})

    lib.utils.unpack_named(_ENV, self.axes)

    local function ccw36(fix)
      return rot{fix=fix, angle=pi/5}
    end
    local function diag_cut(fix, a, _b)
      return ccw36(fix):transform(plane(a.vector.unit * d))
    end

    local UF_edge_region = F(1) & ~L(1) & ~R(1)
    for t, diag_cut, region_to_cut in shape.sym:orbit(diag_cut(F, R, U), UF_edge_region) do
      self:slice(diag_cut, { region = region_to_cut })
    end

    -- Define twists
    for t, ax, rot in shape.sym.chiral:orbit(F, ccw36(F).rev) do
      self.twists:add(ax, rot, { gizmo_pole_distance = 1 })
    end

    do -- Mark piece types
      local center_region = F(1) & ~symmetry{ccw36(F, R, U)}:orbit(U(1)):union()
      self:mark_piece(center_region, 'center', "Center")

      local inner_triangle_region = F(1) & ~diag_cut(F, R, U).region & ~R(1) & ~U(1)
      self:mark_piece(inner_triangle_region, 'triangle', "Triangle")
      local outer_triangle_region = F(1) & U(1) & diag_cut(F, R, U).region & diag_cut(F, U, L).region
      self:mark_piece(outer_triangle_region, 'triangle')

      local edge_region = ~diag_cut(F, R, U).region & ~diag_cut(F, U, L).region & ~diag_cut(U, F, R).region & ~diag_cut(U, L, F).region
      self:mark_piece(edge_region, 'edge', "Edge")

      self:add_piece_type('oblique', "Oblique")
      local oblique_region = F(1) & U(1) & ~R(1) & ~diag_cut(F, R, U).region & diag_cut(F, U, L).region & diag_cut(F, U, L).region & ~diag_cut(U, L, F).region
      self:mark_piece(oblique_region, 'oblique/left', "Left Oblique")
      self:mark_piece(refl('x'):transform(oblique_region), 'oblique/right', "Right Oblique")

      local wing_region = F(1) & U(1) & ~R(1) & diag_cut(U, L, F).region & diag_cut(F, U, L).region
      self:mark_piece(wing_region, 'wing', "Wing")

      self:mark_piece(U(1) & R(1) & F(1), 'corner', "Corner")

      self:unify_piece_types(shape.sym.chiral) -- chiral because left vs. right obliques
    end
  end,

  tags = {
    author = "Andrew Farkas",
    experimental = true,
  },
}
