defpuzzle {
  id = 'abbott_prism',
  name = 'Abbott Prism',
  author = { 'Andrew Farkas', 'Milo Jacquet' },
  inventor = { 'Milo Jacquet' },
  external = { museum = 9248 },

  construction = 'solid',
  space = euclidean(3).quotient{7, 2},
  twists = 'abbott_prism',
  colors = 'prism.7',

  build = function(space)
    local sym = schlafli{7, 2}
    local m = sym.mirror_basis

    local mat1 = m:thru(1, 2, 1, 2)

    -- Solve for the location of the vertex
    local v = symbolicvec(sym, 'x7o2x')
    local constraints = {
      dot(v, sym:thru(1, 2, 1, 2) * v) ~ dot(v, sym:thru(3) * v),
    }
    local v = solve_system(constraints, v)

    local shapes = space.all
    -- Carve equitorial faces
    shapes:carve{point = v, normal = m * vec(0, 1, 0)}
    -- Carve polar faces
    shapes:carve{point = v, normal = m * vec(0, 0, 1)}

    local twist_axis_vector = m * vec(1, 0, 1) -- tweak nonzero numbers to get similar puzzles
    local twist_cut = sphere{
      center = twist_axis_vector,
      tangent = TODO, -- not sure what goes here
    }

    shapes:cut(twist_cut)
    twistaxis{
      vector = twist_axis_vector,
      cuts = {twist_cut},
      twists = {
        -- TODO: how to specify names for twists?
        --       (figure this out for doctrinaire first)
        jumbling_stops = {
          sym:thru(1) * v,
          sym:thru(2, 1) * v,
          sym:thru(3) * v,
          sym:thru(1, 2, 1, 2) * v,
          sym:thru(2, 1, 2, 1, 2) * v,
        },
      }
    }

    space:unquotient()
  end,
}
