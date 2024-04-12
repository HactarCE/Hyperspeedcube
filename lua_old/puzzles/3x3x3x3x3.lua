common = require('common')

puzzledef{
  id = '3x3x3x3x3',
  name = "3x3x3x3x3",
  ndim = 5,

  build = function()
    for v in cd{4, 3, 3, 3}:expand('oooox') do
      carve(v)
      slice(v / 3)
      add_color(v)
    end
  end,
}
