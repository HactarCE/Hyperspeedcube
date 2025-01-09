-- TODO: Jumbling moves, pending an implementation that doesn't lead to runaway

local radio4 = 1/phi
local radio5 = phi^2/(2+(phi^2))
local radio7 = 5-2*sqrt(5)
local radio8 = 1/phi^2
local radio11 = 1/phi^3
local radio12 = 1/phi^4
local radio13 = 1/((phi^4)+(phi^2))

puzzles:add{
  id = 'eitannebula',
  name = "Eitan's Nebula",
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    local dif = 0.08

    self.axes:add(shape:iter_poles(), {1, radio4+dif, radio4, -radio4, -radio4-dif, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1), 'center', "Center")
    self:mark_piece(U(1) & FR(1) & FL(2), 'center_trap', "Center Trapezoid")
    self:mark_piece(FR(2) & FL(2) & U(1), 'center_rhomb', "Center Rhombus")
    self:mark_piece(UR(1) & UL(1) & F(3) & US(3) & UP(3), 'pentagon', "Pentagon")
    self:mark_piece(US(2) & UP(3) & U(1) & UL(1), 'outer_trap/oleft', "Outer Trapezoid (Left)")
    self:mark_piece(UP(2) & US(3) & U(1) & UR(1), 'outer_trap/oright', "Outer Trapezoid (Right)")
    self:mark_piece(US(2) & UP(2) & U(1), 'outer_rhomb', "Outer Rhombus")
    self:mark_piece(U(1) & FR(2) & ~FL(1) & ~FL(2) & ~UR(1) & ~UR(2), 'edge_triangle/right', "Edge Triangle (Right)")
    self:mark_piece(U(1) & FL(2) & ~FR(1) & ~FR(2) & ~UL(1) & ~UL(2), 'edge_triangle/left', "Edge Triangle (Left)")
    self:mark_piece(UR(2) & FR(2) & U(1), 'inner_wing', "Inner Wing")
    self:mark_piece(U(1) & FR(1) & R(3) & UR(1), 'middle_wing', "Middle Wing")
    self:mark_piece(U(1) & FR(1) & R(2) & UR(1), 'outer_wing', "Outer Wing")
    self:mark_piece(U(1) & F(1) & FR(1) & UR(1) & R(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '4650', '!wca' },

    author = "Jason White",
    inventor = "Eitan Cher",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'eitangalaxy',
  name = "Eitan's Galaxy",
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices
    
    local galaxy = sqrt((6/(47+(21*sqrt(5))))+(((3*sqrt(3))-sqrt(15))/2)^2)

    self.axes:add(shape:iter_poles(), {1, galaxy, radio7, -radio7, -galaxy, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1), 'center', "Center")
    self:mark_piece(U(1) & F(1) & UR(2) & UL(1) & ~L(2) & ~FL(2), 'pentagon', "Pentagon")
    self:mark_piece(U(1) & ~UR(1) & ~F(1) & ~UP(1) & ~UP(2) & ~L(1) & ~L(2), 'kite', "Kite")
    self:mark_piece(UR(2) & UL(2) & FR(2) & FL(2), 'edge', "Edge")
    self:mark_piece(U(1) & F(1) & R(1) & FR(1) & UR(1), 'corner', "Corner")
    self:mark_piece(U(1) & F(1) & R(1) & ~UR(1), 'wing', "Wing")
    self:mark_piece(U(1) & F(1) & UR(2) & UL(1) & ~L(3) & ~FL(3), 'inner_long', "Inner Long")
    self:mark_piece(U(1) & F(1) & UR(1) & ~R(1) & ~FR(1) & ~UL(1) & ~UL(2), 'outer_long', "Outer Long")
    self:mark_piece(U(1) & ~R(1) & ~R(2) & ~UL(1) & ~US(1) & US(2), 'thin/left', "Thin Left")
    self:mark_piece(U(1) & ~L(1) & ~L(2) & ~UR(1) & ~UP(1) & UP(2), 'thin/right', "Thin Right")
    self:mark_piece(U(1) & UL(1) & US(2) & UP(3), 'thick/left', "Thick Left")
    self:mark_piece(U(1) & UR(1) & UP(2) & US(3), 'thick/Right', "Thick Right")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = "Jason White",
    inventor = "Eitan Cher",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio01_5',
  name = "Radiolarian 1.5",
  aliases = {"Radio 1.5", "Radio Canon"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, 0.77, -0.77, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(U(1) & ~F(1) & ~UR(1) & ~UL(1), 'center', "Center")
    self:mark_piece(U(1) & F(1) & ~UR(1) & ~FR(1) & ~UL(1) & ~FL(1), 'edge', "Edge")
    self:mark_piece(U(1) & F(1) & UR(1) & ~FR(1) & ~R(1) & ~UL(1), 'kite', "Kite")
    self:mark_piece(UR(1) & FR(1) & ~R(1), 'Wing', "Wing")
    self:mark_piece(U(1) & F(1) & FR(1) & R(1) & UR(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '4766', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    'canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio02',
  name = "Radiolarian 2",
  aliases = {"Radio 2", "Circo-Radiolarian"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, 1-((1-radio4)*2/3), -1-((1-radio4)*2/3), -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & ~UR(1) & ~FR(1) & ~UL(1) & ~FL(1), 'edge', "Edge")
    self:mark_piece(U(1) & F(1) & UR(1) & ~FR(1) & ~R(1) & ~UL(1), 'kite', "Kite")
    self:mark_piece(UR(1) & FR(1) & ~R(1), 'Wing', "Wing")
    self:mark_piece(U(1) & F(1) & FR(1) & R(1) & UR(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '1746', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio03',
  name = "Radiolarian 3",
  aliases = {"Radio 3"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, 1-((1-radio4)*4/5), -1-((1-radio4)*4/5), -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1), 'center', "Center")
    self:mark_piece(U(1) & F(1) & ~UR(1) & ~FR(1) & ~UL(1) & ~FL(1), 'edge', "Edge")
    self:mark_piece(U(1) & F(1) & UR(1) & ~FR(1) & ~R(1) & ~UL(1), 'pentagon', "Pentagon")
    self:mark_piece(UR(1) & FR(1) & ~R(1), 'Wing', "Wing")
    self:mark_piece(U(1) & F(1) & FR(1) & R(1) & UR(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '1747', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio04',
  name = "Radiolarian 4",
  aliases = {"Radio 4", "Eitan's Star", "DeFTI"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio4, -radio4, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1), 'center', "Center")
    self:mark_piece(U(1) & F(1) & UR(1) & ~FR(1) & ~R(1) & ~UL(1), 'triangle', "Triangle")
    self:mark_piece(UR(1) & FR(1) & ~R(1), 'Wing', "Wing")
    self:mark_piece(U(1) & F(1) & FR(1) & R(1) & UR(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '1844', '!wca' },

    author = "Jason White",
    inventor = "Eitan Cher",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio05',
  name = "Radiolarian 5",
  aliases = {"Radio 5", "Cat's Cradle"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio5, -radio5, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1) & ~R(1) & ~FR(1) & ~UP(1) & ~US(1) & ~FL(1) & ~L(1), 'center', "Center")
    self:mark_piece(UR(1) & UL(1) & FR(1) & FL(1), 'edge', "Edge")
    self:mark_piece(R(1) & UL(1) & ~US(1) & ~FR(1), 'edge_petals/right', "Edge Petal (Right)")
    self:mark_piece(L(1) & UR(1) & ~UP(1) & ~FL(1), 'edge_petals/left', "Edge Petal (Left)")
    self:mark_piece(U(1) & ~UL(1) & ~R(1) & ~FL(1) & ~US(1), 'wing', "Wing")
    self:mark_piece(U(1) & F(1) & FR(1) & R(1) & UR(1) & ~UL(1) & ~FL(1) & ~DR(1) & ~S(1) & ~US(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '3632', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio06',
  name = "Radiolarian 6",
  aliases = {"Radio 6", "Radio Web"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio7+0.02, -(radio7+0.02), -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1) & ~R(1) & ~FR(1) & ~UP(1) & ~US(1) & ~FL(1) & ~L(1), 'center', "Center")
    self:mark_piece(UR(1) & UL(1) & FR(1) & FL(1), 'edge', "Edge")
    self:mark_piece(R(1) & UL(1) & ~US(1) & ~FR(1), 'edge_petals/right', "Edge Petal (Right)")
    self:mark_piece(L(1) & UR(1) & ~UP(1) & ~FL(1), 'edge_petals/left', "Edge Petal (Left)")
    self:mark_piece(U(1) & ~UL(1) & ~R(1) & ~FL(1) & ~US(1), 'wing', "Wing")
    self:mark_piece(UL(1) & FR(1) & R(1), 'triangle', "Triangle")
    self:mark_piece(U(1) & F(1) & FR(1) & R(1) & UR(1) & ~UL(1) & ~FL(1) & ~DR(1) & ~S(1) & ~US(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '3640', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio07',
  name = "Radiolarian 7",
  aliases = {"Radio 7", "Radio Jewel"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio7, -radio7, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1) & ~R(1) & ~FR(1) & ~UP(1) & ~US(1) & ~FL(1) & ~L(1), 'center', "Center")
    self:mark_piece(UR(1) & UL(1) & FR(1) & FL(1), 'edge', "Edge")
    self:mark_piece(R(1) & UL(1) & ~US(1) & ~FR(1), 'edge_petals/right', "Edge Petal (Right)")
    self:mark_piece(L(1) & UR(1) & ~UP(1) & ~FL(1), 'edge_petals/left', "Edge Petal (Left)")
    self:mark_piece(UL(1) & FR(1) & R(1), 'triangle', "Triangle")
    self:mark_piece(U(1) & F(1) & FR(1) & R(1) & UR(1) & ~UL(1) & ~FL(1) & ~DR(1) & ~S(1) & ~US(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '3639', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio08',
  name = "Radiolarian 8",
  aliases = {"Radio 8", "Radio Jam"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio7-0.06, -(radio7-0.06), -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1) & ~R(1) & ~FR(1) & ~UP(1) & ~US(1) & ~FL(1) & ~L(1), 'center', "Center")
    self:mark_piece(UR(1) & UL(1) & FR(1) & FL(1) & ~R(1) & ~L(1), 'edge', "Edge")
    self:mark_piece(R(1) & UL(1) & ~US(1) & ~FR(1), 'edge_petals/right', "Edge Petal (Right)")
    self:mark_piece(L(1) & UR(1) & ~UP(1) & ~FL(1), 'edge_petals/left', "Edge Petal (Left)")
    self:mark_piece(UL(1) & FR(1) & R(1) & ~FL(1) & ~US(1), 'pentagon', "Pentagon")
    self:mark_piece(FL(1) & R(1) & UL(1), 'wing', "Wing")
    self:mark_piece(U(1) & ~UL(1) & ~FL(1) & ~DR(1) & ~S(1) & ~US(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '3685', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio09',
  name = "Radiolarian 9",
  aliases = {"Radio 9", "Radio Crystal"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, 1/3, -1/3, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(R(1) & L(1), 'edge', "Edge")
    self:mark_piece(UL(1) & FR(1) & R(1) & ~FL(1) & ~US(1), 'triangle', "Triangle")
    self:mark_piece(FL(1) & R(1) & UL(1) & ~L(1), 'wing', "Wing")
    self:mark_piece(U(1) & ~UL(1) & ~FL(1) & ~DR(1) & ~S(1) & ~US(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '3732', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio10',
  name = "Radiolarian 10",
  aliases = {"Radio 10", "Radio Nova"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, 1/3-0.05, -(1/3-0.05), -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(FR(1) & FL(1) & US(1) & UP(1) & R(1) & L(1), 'center', "Center")
    self:mark_piece(R(1) & L(1) & ~FR(1) & FL(1), 'center_petals/right', "Center Petal (Right)")
    self:mark_piece(R(1) & L(1) & ~FL(1) & FR(1), 'center_petals/left', "Center Petal (Left)")
    self:mark_piece(R(1) & L(1) & ~FR(1) & ~FL(1), 'kite', "Kite")
    self:mark_piece(R(1) & L(1) & ~UP(1) & ~US(1) & ~DR(1) & ~DL(1), 'edge', "Edge")
    self:mark_piece(UL(1) & FR(1) & R(1) & ~FL(1) & ~US(1), 'triangle', "Triangle")
    self:mark_piece(FL(1) & R(1) & UL(1) & ~L(1) & ~US(1) & ~DR(1), 'wing', "Wing")
    self:mark_piece(U(1) & ~UL(1) & ~FL(1) & ~DR(1) & ~S(1) & ~US(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio11',
  name = "Radiolarian 11",
  aliases = {"Radio 11", "Radio Star"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio11, -radio11, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(FR(1) & FL(1) & US(1) & UP(1) & R(1) & L(1), 'center', "Center")
    self:mark_piece(R(1) & L(1) & ~FR(1) & FL(1), 'center_petals/right', "Center Petal (Right)")
    self:mark_piece(R(1) & L(1) & ~FL(1) & FR(1), 'center_petals/left', "Center Petal (Left)")
    self:mark_piece(R(1) & L(1) & ~FR(1) & ~FL(1), 'kite', "Kite")
    self:mark_piece(R(1) & L(1) & ~UP(1) & ~US(1) & ~DR(1) & ~DL(1), 'edge', "Edge")
    self:mark_piece(FL(1) & R(1) & UL(1) & ~L(1) & ~US(1) & ~DR(1), 'wing', "Wing")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '4538', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio12',
  name = "Radiolarian 12",
  aliases = {"Radio 12", "Radio Nebula"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio12, -radio12, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(FR(1) & FL(1) & US(1) & UP(1) & R(1) & L(1), 'center', "Center")
    self:mark_piece(R(1) & L(1) & ~FR(1) & FL(1), 'center_petals/right', "Center Petal (Right)")
    self:mark_piece(R(1) & L(1) & ~FL(1) & FR(1), 'center_petals/left', "Center Petal (Left)")
    self:mark_piece(R(1) & L(1) & ~FR(1) & ~FL(1) & ~BR(1) & ~BL(1), 'hexagon', "Hexagon")
    self:mark_piece(R(1) & L(1) & ~UP(1) & ~US(1) & ~DR(1) & ~DL(1), 'edge', "Edge")
    self:mark_piece(FL(1) & R(1) & UL(1) & ~L(1) & ~US(1) & ~DR(1), 'inner_wing', "Inner Wing")
    self:mark_piece(R(1) & L(1) & ~F(1), 'outer_wing', "Outer Wing")
    self:mark_piece(F(1) & R(1) & L(1) & BR(1) & BL(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '3934', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio13',
  name = "Radiolarian 13",
  aliases = {"Radio 13", "Radio Gem"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio13, -radio13, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(FR(1) & FL(1) & US(1) & UP(1) & R(1) & L(1), 'center', "Center")
    self:mark_piece(R(1) & L(1) & ~FR(1) & FL(1), 'center_petals/right', "Center Petal (Right)")
    self:mark_piece(R(1) & L(1) & ~FL(1) & FR(1), 'center_petals/left', "Center Petal (Left)")
    self:mark_piece(R(1) & L(1) & ~FR(1) & ~FL(1) & ~BR(1) & ~BL(1), 'rhombus', "Rhombus")
    self:mark_piece(R(1) & L(1) & ~UP(1) & ~US(1) & ~DR(1) & ~DL(1), 'edge', "Edge")
    self:mark_piece(R(1) & L(1) & ~F(1), 'wing', "Wing")
    self:mark_piece(F(1) & R(1) & L(1) & BR(1) & BL(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '4243', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio14',
  name = "Radiolarian 14",
  aliases = {"Radio 14", "Radio Fathom"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, 0.05, -0.05, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(FR(1) & FL(1) & US(1) & UP(1) & R(1) & L(1), 'center', "Center")
    self:mark_piece(R(1) & L(1) & ~FR(1) & FL(1) & ~BL(1), 'center_petals/right', "Center Petal (Right)")
    self:mark_piece(R(1) & L(1) & ~FL(1) & FR(1) & ~BR(1), 'center_petals/left', "Center Petal (Left)")
    self:mark_piece(R(1) & L(1) & ~FR(1) & ~FL(1) & ~BR(1) & ~BL(1), 'rhombus', "Rhombus")
    self:mark_piece(R(1) & L(1) & ~UP(1) & ~US(1) & ~DR(1) & ~DL(1), 'edge', "Edge")
    self:mark_piece(R(1) & L(1) & ~F(1) & B(1), 'inner_wing', "Inner Wing")
    self:mark_piece(R(1) & L(1) & ~F(1) & ~B(1), 'outer_wing', "Outer Wing")
    self:mark_piece(F(1) & R(1) & L(1) & BR(1) & BL(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '4242', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}
puzzles:add{
  id = 'radio15',
  name = "Radiolarian 15",
  aliases = {"Radio 15", "Radio Chop"},
  version = '1.0.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, 0, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type
    self:mark_piece(FR(1) & FL(1) & US(1) & UP(1) & R(1) & L(1), 'center', "Center")
    self:mark_piece(R(1) & L(1) & ~F(1) & B(1), 'wing', "Wing")
    self:mark_piece(F(1) & R(1) & L(1) & BR(1) & BL(1), 'corner', "Corner")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym.chiral)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = '4490', '!wca' },

    author = "Jason White",
    inventor = "Jason Smith",

    'type/puzzle',
    'shape/3d/platonic/icosahedron',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire', -- pending jumbling
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/icosahedral', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {"face", "facet"},
    'experimental',
    '!canonical',
    'family/radiolarian',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}