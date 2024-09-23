puzzles:add{
  id = 'test_puzzle',
  name = "Test Puzzle",
  version = '0.0.1',
  ndim = 3,
  build = function(p) p:carve(cd'bc3':orbit(cd'bc3'.oox.unit)) end,
}
