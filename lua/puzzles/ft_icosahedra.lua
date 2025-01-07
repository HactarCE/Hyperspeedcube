-- TODO: Jumbling moves, pending an implementation that doesn't lead to runaway

local radio4 = 1/phi
local radio7 = 5-2*sqrt(5)
local radio8 = 1/phi^2
local radio11 = 1/phi^3

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
    '!experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio1_5',
  name = "Radiolarian 1.5",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio2',
  name = "Radiolarian 2",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio3',
  name = "Radiolarian 3",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio4',
  name = "Radiolarian 4",
  version = '0.1.0',
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
  id = 'radio6',
  name = "Radiolarian 6",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio7',
  name = "Radiolarian 7",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio8',
  name = "Radiolarian 8",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio9',
  name = "Radiolarian 9",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio10',
  name = "Radiolarian 10",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio11',
  name = "Radiolarian 11",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio12',
  name = "Radiolarian 12",
  version = '0.1.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices

    self.axes:add(shape:iter_poles(), {1, radio11-0.09, -(radio11-0.09), -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- TODO: Mark one copy of each piece-type

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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}

puzzles:add{
  id = 'radio14',
  name = "Radiolarian 14",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}
puzzles:add{
  id = 'radio15',
  name = "Radiolarian 15",
  version = '0.1.0',
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
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting', -- pending jumbling
  },
}