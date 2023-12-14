common = require('common')

puzzledef{
  id = '3x3x3x3',
  name = "3x3x3x3",
  aliases = {
    "{4, 3, 3} 3",
    "3^4",
    "Magic Cube 4D",
    "Rubik's Hypercube",
  },
  ndim = 3,
  meta = {
    id = '3x3x3x3',
    author = "Andrew Farkas",

    year = 1988,
    inventors = {
      "Andrey Astrelin",
      "Don Hatch",
      "Melinda Green",
    },

    external = {
      mc4d = "3x3x3x3",
    },
    description = [[
      Invented indepedently by Andrey Astrelin, and Melinda Green & Don Hatch
    ]],
  },

  properties = {
    shallow_cut = true,
    doctrinaire = true,
  },

  build = function()
    for v in cd{4, 3, 3}:expand('ooox') do
      carve(v)
      -- slice(v / 3)
    end

    if true then return end

    define_facets(common.facets.hypercube())
    define_axes(common.axes.hypercubic{1/3, -1/3})

    R, L = axes.R, axes.L
    U, D = axes.U, axes.D
    F, B = axes.F, axes.B
    O, I = axes.O, axes.I

    define_twists{
      axis = I,
      generator = rot{plane=I, from=F, to=U},
      multipliers = 'auto',
      symmetry = sym,
      name = function(t, m)
        return (t * I).name .. (t * R).name .. common.multiplier_suffix(m)
      end,
    }

    function get_twist_fn(pos, neg)
      return function(ax)
        local other = pos
        if ax == pos then other = O   end
        if ax == neg then other = I   end
        if ax == O   then other = neg end
        return twist(ax .. other.name)
      end
    end
    define_twist_directions{
      ["x"] = get_twist_fn(R, L),
      ["y"] = get_twist_fn(U, D),
      ["z"] = get_twist_fn(F, B),
      multipliers = {2, 1, -1, -2},
    }
    define_piece_types{
      symmetry = {4, 3},
      corners = R(1) & U(1) & F(1) & I(1),
      edges   = R(1) & U(1) & F(1),
      ridges  = R(1) & U(1),
      centers = R(1),
    }
  end,
}
