local function add_symmetric_colors(...)
  for v in ... do
    add_color(v)
  end
end

function cube(radius)
  add_symmetric_colors(cd{4, 3}:expand('oox'))
  name_colors{'Right', 'Up', 'Left', 'Down', 'Front', 'Back'}
  reorder_colors{'Right', 'Left', 'Up', 'Down', 'Front', 'Back'}
  set_default_colors{'red', 'orange', 'white', 'yellow', 'green', 'blue'}
end

function dodecahedron(radius)
  add_symmetric_colors(cd{5, 3}:expand('oox'))
end

function hypercube(radius)
  add_symmetric_colors(cd{4, 3, 3}:expand('ooox'))
  name_colors{'Right', 'Up', 'Left', 'Down', 'Front', 'Back', 'Out', 'In'}
  reorder_colors{'Right', 'Left', 'Up', 'Down', 'Front', 'Back', 'Out', 'In'}
  set_default_colors{'red', 'orange', 'white', 'yellow', 'green', 'blue', 'pink', 'purple'}
end
