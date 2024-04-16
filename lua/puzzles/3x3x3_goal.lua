sym = cd{4, 3}

shapes:add('cube', {
  ndim = 3,
  symmetry = cd{4, 3},
  build = function(shape)
    shape:carve(sym:vec('oox'):normalized())

    shape.colors:rename{'Front', 'Up', 'Right', 'Left', 'Down', 'Back'}
    shape.colors:reorder{'Right', 'Left', 'Up', 'Down', 'Front', 'Back'}
    shape.colors:set_defaults{'red', 'orange', 'white', 'yellow', 'green', 'blue'}
  end,
})

axis_systems:add('cubic', {
  ndim = 3,
  symmetry = cd{4, 3},
  build = function(axes)
    axes:add(sym:vec('oox'))
    axes:rename{'F', 'U', 'R', 'L', 'D', 'B'}
    axes:reorder{'R', 'L', 'U', 'D', 'F', 'B'}
  end,
})

twist_systems:add('ft_cubic', {
  ndim = 3,
  axes = 'cubic',
  symmetry = cd{4, 3},
  build = function(twists)
    local R = twists.axes.R
    local U = twists.axes.U
    local F = twists.axes.F
    local twist_rot = rot{fix = U, from = R, to = F}
    for transform, axis, twist_rot in cd{4, 3}:chiral():orbit(U, twist_rot) do
      twists:add({
        axis_prefix = true, -- this is default
        name = "", -- this is default
        inv_name = "'", -- this is default
        inverse = true, -- this is default
        axis = axis,
        transform = twist_rot,
        -- angle = math.tau / 4, -- can be used instead of `transform` in 3D
      })
    end
    -- twists.directions:add("CW", {twist = function(ax) return ax.name end})
    -- twists.directions:add("CCW", {twist = function(ax) return ax.name .. "'" end})
    -- twists.directions:add("180 CW", {twist = function(ax) return {ax.name, ax.name} end})
    -- twists.directions:add("180 CCW", {twist = function(ax) return {ax.name .. "'", ax.name .. "'"} end})
  end,
})

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

    -- p.piece_types:add('corner', {symmetry = cd{4, 3}, seed = R(1) & U(1) & F(1)})
    -- p.piece_types:add('edge', {symmetry = cd{4, 3}, seed = R(1) & U(1)})
    -- p.piece_types:add('center', {symmetry = cd{4, 3}, seed = R(1)})
  end,
})
