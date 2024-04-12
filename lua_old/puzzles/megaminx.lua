common = require('common')

puzzledef{
  id = 'megaminx',
  name = "Megaminx",
  aliases = {
    "Hungarian Supernova",
    "Magic Dodecahedron",
    "Rubik's Megaminx",
  },
  ndim = 3,
  meta = {
    id = 'megaminx',
    author = "Andrew Farkas",

    year = 1982,
    inventors = {
      "Szlivka Ferenc",
      "Christoph Bandelow",
      "Benjamin R. Halpern",
    },

    family = "wca",
    external = {
      pcubes = "Megaminx",
      gelatinbrain = "1.1.1",
      museum = 651,
    },
  },

  properties = {
    shallow_cut = true,
    doctrinaire = true,
  },

  build = function()
    local D = 1 / math.phi

    common.carve_and_slice_face_turning({5, 3}, D)
    common.colors.dodecahedron()

    -- define_facets(common.facets.dodecahedron())
    -- define_axes(common.axes.dodecahedral{D, -D}) -- TODO: check cut depth
    -- define_twists(common.symmetric_twists_3d({5, 3}, 'F', 'U', 'R'))
    -- define_twist_directions(common.twist_directions_2d(5))
  end,
}
