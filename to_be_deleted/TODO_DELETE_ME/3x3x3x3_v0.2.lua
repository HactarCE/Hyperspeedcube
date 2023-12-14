puzzledef{
  name = '3x3x3x3 v0.2',
  ndim = 4,
  build = function(s)
    local sym = schlafli{4, 3, 3}

    s:fold(sym)
    local v = sym:mvec('w')
    print("Carving facet plane")
    s:carve{pole = v}
    print("Slicing middle plane")
    local twist_axis = s:sliceaxis{normal = v, distance = 1/3}
    print("Expanding symmetry")
    s:unfold()

    print("Defining twist")
    twist_axis:add_twist(rot{from = v, to = sym:thru(v, 2)})
    print("Expanding twist symmetry")
    twist_axis:expand(sym)

    print("Labeling colors and twist axes")
    local generated_order = chartable("RULDFB")
    relabel_colors(order)
    relabel_twists(order)

    print("Reordering colors and twist axes")
    local canonical_order = chartable("RLUDFB")
    reorder_colors(canonical_order)
    reorder_twists(canonical_order)

    default_colors{
      R = 'red', L = 'orange',
      U = 'white', D = 'yellow',
      F = 'green', B = 'blue',
    }
  end,
}
