puzzledef{
  name = '3x3x3',
  aliases = {"Rubik's Cube", '3^3'},
  designer = 'Ernő Rubik',
  author = 'Andrew Farkas',

  tags = {
    'shallow-cut',
    'doctrinaire',
    'wca',
  },

  shape = 'cube',
  build = function(s)
    local poles = cd(symbol):expand(vec(0, 0, 1/3))
    for _, v in ipairs(poles) do
      s:cut(plane{pole = v})
    end
    return s
  end,
  twists = {'cubic', r = 1/3},
}
