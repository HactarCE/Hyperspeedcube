puzzles:add{
  id = 'eitangalaxy',
  name = "Eitan's Galaxy",
  version = '0.1.0',
  ndim = 3,
  colors = 'icosahedron',
  build = function(self)
    local sym = cd'h3'
    local shape = lib.symmetries.icosahedral.icosahedron()

    self:carve(shape:iter_poles())

    -- Define axes and slices
    local radio7 = 5-2*sqrt(5)
    local radio3 = sqrt((6/(47+(21*sqrt(5))))+(((3*sqrt(3))-sqrt(15))/2)^2)

    self.axes:add(shape:iter_poles(), {1, radio3, radio7, -radio7, -radio3, -1})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.xoo.unit], sym:thru(3, 2)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes)

    -- Mark one copy of each piece-type
    self:mark_piece(U(1) & F(1) & R(1) & FR(1) & UR(1), 'corner', "Corner")
    self:mark_piece(U(1) & F(1) & R(1) & ~UR(1), 'wing', "Wing")
    self:mark_piece(U(1) & F(1) & UR(1) & UL(1), 'center', "Center")
    self:mark_piece(U(1) & F(1) & UR(2) & UL(1) & ~L(2) & ~FL(2), 'pentagon', "Pentagon")
    self:mark_piece(U(1) & F(1) & UR(2) & UL(1) & ~L(3) & ~FL(3), 'inner_long', "Inner Long")
    self:mark_piece(UR(2) & UL(2) & FR(2) & FL(2), 'edge', 'Edge')

    -- Pattern piece-types around the puzzle
    --self:unify_piece_types(sym)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

    author = "Jason White",
    inventor = "Eitan Cher",

    'type/puzzle',
    'shape/TODO',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { TODO, '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', '!multi_facet_per' },
    completeness = { '!super', '!real', '!laminated', '!complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = nil,
    'experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}