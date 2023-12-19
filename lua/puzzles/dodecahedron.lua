common = require('common')

puzzledef{
  id = 'dodecahedron',
  name = "Dodecahedron",
  ndim = 3,
  build = function()
    for v in cd{5, 3}:expand('oox') do
      carve(v)
      add_color(v)
    end
  end,
}
