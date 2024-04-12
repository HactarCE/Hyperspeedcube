common = require('common')

puzzledef{
  id = "cubetest",
  name = "** cubetest",
  ndim = 3,

  build = function()
    for v in cd{4, 3}:expand('oox') do
      carve(v)
      add_color(v)
    end
    slice('x')
  end,
}
