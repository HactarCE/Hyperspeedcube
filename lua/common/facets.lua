function cube(radius)
  return {
    id = 'cube',
    symmetry = {4, 3},
    seed = schlafli{4, 3}:vec('z') * (radius or 1),
    names = {'Right', 'Up', 'Left', 'Down', 'Front', 'Back'},
    order = {'Right', 'Left', 'Up', 'Down', 'Front', 'Back'},
    colors = {'red', 'orange', 'white', 'yellow', 'green', 'blue'},
  }
end

function dodecahedron(radius)
  return {
    id = 'dodecahedron',
    symmetry = {5, 3},
    seed = schlafli{5, 3}:vec('z') * (radius or 1),
    -- TODO megaminx face names (in order) & colors
  }
end

function hypercube(radius)
  return {
    id = 'hypercube',
    symmetry = {4, 3, 3},
    seed = schlafli{4, 3, 3}:vec('w') * (radius or 1),
    names = {'Right', 'Up', 'Left', 'Down', 'Front', 'Back', 'Out', 'In'},
    order = {'Right', 'Left', 'Up', 'Down', 'Front', 'Back', 'Out', 'In'},
    colors = {'red', 'orange', 'white', 'yellow', 'green', 'blue', 'pink', 'purple'},
  }
end
