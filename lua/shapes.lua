local TAGS_FOR_ALL_SHAPES = {
  -- builtin = nil,
  external = { '!gelatinbrain', '!hof', '!mc4d', '!museum', '!wca' },

  -- author = nil,
  '!inventor',

  -- 'shape/TODO',
  algebraic = {
    '!doctrinaire', '!pseudo',
    '!abelian', '!fused', '!orientations/non_abelian', '!trivial', '!weird_orbits',
  },
  '!axes', axes = { '!hybrid', '!multicore' },
  colors = { '!multi_facet_per', '!multi_per_facet' },
  completeness = { '!complex', '!laminated', '!real', '!super' },
  cuts = { '!depth', '!stored', '!wedge' },
  '!experimental',
  '!canonical',
  '!family',
  '!meme',
  '!shapeshifting',
  '!turns_by',
  '!variant',
}

local function add_basic_shape(shape_version, hsc_version, author, shape_tag_subpath, shape)
  local ndim = shape.sym.ndim
  puzzles:add{
    id = shape.name:lower():gsub('-', '_'),
    version = shape_version,
    name = shape.name,
    aliases = shape.aliases,
    colors = shape.face_colors,
    ndim = ndim,
    build = function(self)
      self:carve(shape:iter_poles())
      lib.piece_types.mark_everything_core(self)
    end,
    tags = merge_tags(
      {
        builtin = hsc_version,
        author = author,
        'type/shape',
        'shape/' .. ndim .. 'd/' .. shape_tag_subpath,
      },
      TAGS_FOR_ALL_SHAPES
    ),
  }
end

local v = '1.0.0'
local hsc_v = '2.0.0'
local author = "Andrew Farkas"
add_basic_shape(v, hsc_v, author, 'platonic/tetrahedron', lib.symmetries.tetrahedral.tetrahedron())
add_basic_shape(v, hsc_v, author, 'platonic/cube', lib.symmetries.bc3.cube())
add_basic_shape(v, hsc_v, author, 'platonic/octahedron', lib.symmetries.bc3.octahedron())
add_basic_shape(v, hsc_v, author, 'platonic/dodecahedron', lib.symmetries.h3.dodecahedron())
add_basic_shape(v, hsc_v, author, 'platonic/icosahedron', lib.symmetries.h3.icosahedron())
add_basic_shape(v, hsc_v, author, 'platonic/hypercube', lib.symmetries.bc4.hypercube())
add_basic_shape(v, hsc_v, author, 'platonic/hypercube', lib.symmetries.bc5.hypercube())

puzzle_generators:add{
  id = 'duoprism',
  version = '1.0.0',
  name = 'Polygonal Duoprism',
  params = {
    lib.puzzles.prisms.PARAMS.polygon_size("Polygon A"),
    lib.puzzles.prisms.PARAMS.polygon_size("Polygon B"),
  },
  gen = function(params)
    local n, m = table.unpack(params)
    if n < m then
      return 'duoprism', {m, n}
    end
    return {
      name = string.format("{%d}x{%d} Duoprism", n, m),
      ndim = 4,
      colors = string.format('duoprism:%d,%d', n, m),
      build = function(self)
        local polygon_a = lib.symmetries.polygonal.ngon(n, 1, 'xy')
        local polygon_b = lib.symmetries.polygonal.ngon(m, 1, 'zw')

        self:carve(polygon_a:iter_poles('A'))
        self:carve(polygon_b:iter_poles('B'))

        lib.piece_types.mark_everything_core(self)
      end,
      tags = { 'type/shape' }
    }
  end,

  examples = {
    {
      params = {100, 4},
      tags = { meme = true },
      aliases = "Onehundredagonal Duoprism",
    },
  },

  tags = merge_tags(
    {
      builtin = hsc_v,
      author = author,
      'shape/4d/duoprism',
    },
    TAGS_FOR_ALL_SHAPES
  ),
}
