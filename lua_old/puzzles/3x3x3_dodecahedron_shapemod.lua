common = require('common')

puzzledef{
  id = "3x3x3_dodecahedron_shapemod",
  name = "3x3x3 Dodecahedron Shapemod",
  ndim = 3,
  build = function()
    for v in cd{5, 3} 'oox' do
      carve(v:normalized())
      add_color(v:normalized())
    end
    for v in cd{4, 3} 'oox' do
      slice(v:normalized() / 3)
    end
  end,
}
