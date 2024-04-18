sym = cd{4, 3}

puzzles:add('3x3x3', {
  name = "3x3x3",
  aliases = {
    "{4, 3} 3",
    "3^3",
    "Rubik's Cube",
  },
  ndim = 3,
  meta = {
    id = '3x3x3',
    author = "Andrew Farkas",

    year = 1970,
    inventor = "Ernő Rubik",

    family = "wca",
    external = {
      pcubes = "3x3x3",
      gelatinbrain = "3.1.2",
      museum = 2968,
    },
  },

  properties = {
    shallow_cut = true,
    doctrinaire = true,
  },

  shape = 'cube',
  twists = 'ft_cubic',

  build = function(p)
    for _, ax in ipairs(p.twists.axes) do
      local cut = plane{normal = ax.vector, distance = 1/3}
      local opposite_cut = plane{normal = ax.vector, distance = -1/3}
      p.shape:slice(cut)
      ax.layers:add(cut)
      ax.layers:add(opposite_cut, nil)
    end

    -- p.twists.aliases:add("M", {2, "L"})
    -- p.twists.aliases:add("E", {2, "D"})
    -- p.twists.aliases:add("S", {2, "F"})
    -- p.twists.aliases:add_wide_move_suffix("w")

    -- local R = p.twists.axes.R
    -- local U = p.twists.axes.U
    -- local F = p.twists.axes.F

    -- p.piece_types:add('corner', {symmetry = sym, seed = R(1) & U(1) & F(1)})
    -- p.piece_types:add('edge', {symmetry = sym, seed = R(1) & U(1)})
    -- p.piece_types:add('center', {symmetry = sym, seed = R(1)})
  end,
})
