puzzles:add{
  id = 'complex_3x3x3',
  name = "Complex 3x3x3",
  version = '1.0.0',
  ndim = 3,
  colors = 'cube',
  remove_internals = false,
  build = function(self)
    local sym = cd'bc3'
    local shape = lib.symmetries.cubic.cube()
    self:carve(shape:iter_poles())

    -- Define axes and slices
    self.axes:add(shape:iter_poles(), {3/5, -1/5})

    -- Define twists
    for _, axis, twist_transform in sym.chiral:orbit(self.axes[sym.oox.unit], sym:thru(2, 1)) do
      self.twists:add(axis, twist_transform, {gizmo_pole_distance = 1})
    end

    --Give axes labels for filters, twists, and to simplify following step
    lib.utils.unpack_named(_ENV, self.axes) 

    -- Add super-stickers on internal faces
    for i=3,-3,-2 do
        self:slice(plane(vec('x'), i/5), {stickers=self.colors.R})
        self:slice(plane(vec('x')*-1, i/5), {stickers=self.colors.L})
        self:slice(plane(vec('y'), i/5), {stickers=self.colors.U})
        self:slice(plane(vec('y')*-1, i/5), {stickers=self.colors.D})
        self:slice(plane(vec('z'), i/5), {stickers=self.colors.F})
        self:slice(plane(vec('z')*-1, i/5), {stickers=self.colors.B})
    end

    -- Mark one copy of each piece-type
    self:mark_piece(~R(1) & ~L(1) & ~U(1) & ~D(1) & ~F(1) & ~B(1), 'core', "Core")
    self:mark_piece(R(1) & ~L(1) & ~U(1) & ~D(1) & ~F(1) & ~B(1), 'center', "Center")
    self:mark_piece(R(1) & ~L(1) & U(1) & ~D(1) & ~F(1) & ~B(1), 'edge', "Edge")
    self:mark_piece(R(1) & L(1) & ~U(1) & ~D(1) & ~F(1) & ~B(1), 'axle', "Axle")
    self:mark_piece(R(1) & ~L(1) & U(1) & ~D(1) & F(1) & ~B(1), 'corner', "Corner")
    self:mark_piece(R(1) & L(1) & U(1) & ~D(1) & ~F(1) & ~B(1), 'triwall', "Triwall")
    self:mark_piece(R(1) & L(1) & U(1) & ~D(1) & F(1) & ~B(1), 'antiedge', "Anti-Edge")
    self:mark_piece(R(1) & L(1) & U(1) & D(1) & ~F(1) & ~B(1), 'antiaxle', "Anti-Axle")
    self:mark_piece(R(1) & L(1) & U(1) & ~D(1) & F(1) & B(1), 'anticenter', "Anti-Center")
    self:mark_piece(R(1) & L(1) & U(1) & D(1) & F(1) & B(1), 'anticore', "Anti-Core")

    -- Pattern piece-types around the puzzle
    self:unify_piece_types(sym)

  end,

  tags = {
    builtin = false,
    external = { '!gelatinbrain', '!hof', '!mc4d', museum = 6777, '!wca' },

    author = "Jason White",
    '!inventor',

    'type/puzzle',
    'shape/3d/platonic/cube',
    algebraic = {
      'doctrinaire', 'pseudo/doctrinaire',
      '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
    },
    axes = { '3d/elementary/cubic', '!hybrid', '!multicore' },
    colors = { '!multi_per_facet', 'multi_facet_per' },
    completeness = { 'super', '!real', '!laminated', 'complex' },
    cuts = { '!depth', '!stored', '!wedge' },
    turns_by = {'face', 'facet'},
    'experimental',
    '!canonical',
    '!family',
    '!variant',
    '!meme',
    '!shapeshifting',
  },
}
