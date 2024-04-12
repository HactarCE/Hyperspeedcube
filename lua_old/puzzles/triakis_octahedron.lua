common = require('common')

puzzledef{
  id = 'triakis_octahedron',
  name = "Triakis Octahedron",
  ndim = 3,
  meta = {
    id = 'triakis_octahedron',
    author = "Milo Jacquet",
  },

  build = function()
    for v in cd{4, 3}:expand('xxo') do
      carve(v)
      add_color(v)
    end
  end,
}
