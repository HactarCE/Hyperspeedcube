common = require('common')

puzzledef{
  id = "octahedron",
  name = "Octahedron",

  ndim = 3,

  build = function()
    for v in cd{3, 4}:expand('oox') do
      carve(v)
      add_color(v)
    end
  end,
}
