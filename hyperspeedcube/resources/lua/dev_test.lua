puzzles:add('test_puzzle', {
  name = "Test Puzzle",
  ndim = 3,
  build = function(p) p:carve(cd'bc3':orbit(cd'bc3'.oox.unit)) end,
})
