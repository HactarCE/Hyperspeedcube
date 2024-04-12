common = require('common')

puzzledef{
  id = 'dino_cube',
  name = "Dino Cube",
  ndim = 3,
  meta = {
    id = 'dino_cube',
    author = "Andrew Farkas",

    year = 1995,
    inventor = "S. Y. Liou",

    external = {
      pcubes = "Dino Cube",
      gelatinbrain = "3.2.4",
      museum = 605,
    },
  },

  properties = {
    cut_to_adjacent = true,
    doctrinaire = true,
  },

  build = function()
    for v in cd{4, 3}:expand('oox') do
      carve(v:normalized())
      add_color(v:normalized())
    end
    for v in cd{4, 3}:expand('xoo') do
      slice(v:normalized() / math.sqrt(3))
    end

    if true then return end

    define_axes{
      id = 'octahedral',
      symmetry = {4, 3},
      seed = svec(1, 1, 1),
      depths = {D, -D},
      letters = {'R', 'U', 'L', 'D', 'F', 'B'}, -- TODO: figure out this order
      order = {'R', 'L', 'U', 'D', 'F', 'B'}, -- TODO: figure out this order
    }
    define_twists(common.symmetric_twists_3d({4, 3}, 'F', 'U', 'R'))
    define_twist_directions(common.twist_directions_2d(4))
  end,
}
