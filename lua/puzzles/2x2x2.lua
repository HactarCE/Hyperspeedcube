common = require('common')

puzzledef{
  id = "2x2x2",
  name = "2x2x2",
  aliases = {
    "{4, 3} 2",
    "2^3",
    "Rubik's Pocket Cube",
  },
  ndim = 3,
  meta = {
    id = '2x2x2',
    author = "Andrew Farkas",

    -- year = 1970, -- TODO: when?
    -- inventor = "Ernő Rubik", -- TODO: who?

    family = "wca",
    external = {
      pcubes = "2x2x2",
      -- gelatinbrain = "3.1.2", -- TODO: which?
      -- museum = 2968, -- TODO: which?
    },
  },

  properties = {
    shallow_cut = true,
    doctrinaire = true,
  },

  build = function()
    common.carve_and_slice_face_turning({4, 3}, 0)
    common.colors.cube()
    if true then return end
    define_axes(common.axes.cubic{0})

    R, U, F = axes.R, axes.U, axes.F

    define_twists(common.symmetric_twists_3d({4, 3}, F, U, R))
    define_twist_directions(common.twist_directions_2d(4))

    define_piece_types{
      symmetry = {4, 3},
      {name = 'corner', R(1) & U(1) & F(1)},
    }
  end,
}
