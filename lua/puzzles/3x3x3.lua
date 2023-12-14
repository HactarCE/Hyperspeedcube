common = require('common')

puzzledef{
  id = "3x3x3",
  name = "3x3x3",
  aliases = {
    "{4, 3} 3",
    "3^3",
    "Rubik's Cube",
  },
  ndim = 3,
  meta = {
    id = '3x3x3',
    author = "Andrew Farkas",

    year = 1970,
    inventor = "Ernő Rubik",

    family = "wca",
    external = {
      pcubes = "3x3x3",
      gelatinbrain = "3.1.2",
      museum = 2968,
    },
  },

  properties = {
    shallow_cut = true,
    doctrinaire = true,
  },

  build = function()
    common.carve_and_slice_face_turning({4, 3}, 1/3)

    if true then return end

    define_facets(common.facets.cube())
    define_axes(common.axes.cubic{1/3, -1/3})

    R, U, F = axes.R, axes.U, axes.F

    define_twists(common.symmetric_twists_3d({4, 3}, F, U, R))
    define_twist_directions(common.twist_directions_2d(4))

    define_notation_aliases{
      M = {2, 'L'},
      E = {2, 'D',},
      S = {2, 'F'},
    }
    for ax in pairs(axes) do
      define_notation_aliases{
        [ax .. w] = {2, ax},
      }
    end

    define_piece_types{
      symmetry = {4, 3},
      {name = 'corner', R(1) & U(1) & F(1)},
      {name = 'edge',   R(1) & U(1)},
      {name = 'center', R(1)},
    }
  end,
}
