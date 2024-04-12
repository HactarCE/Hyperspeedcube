CUBIC = {
  id = 'cubic',
  symmetry = {4, 3},
  seed = schlafli{4, 3}:vec('z'),
  letters = {'R', 'U', 'L', 'D', 'F', 'B'},
  order = {'R', 'L', 'U', 'D', 'F', 'B'},
  rotations = {
    x = rot{from = vec('z'), to = vec('y')},
    y = rot{from = vec('x'), to = vec('z')},
    z = rot{from = vec('y'), to = vec('x')},
  },
}

function dodecahedral(depths, seed)
  return {
    id = 'dodecahedral',
    symmetry = {5, 3},
    seed = seed or schlafli{5, 3}:mvec('z'),
    depths = depths,
    -- TODO megaminx face names (in order)
    -- TODO megaminx rotations
  }
end

function hypercubic(depths, seed)
  return {
    id = 'hypercubic',
    symmetry = {4, 3, 3},
    seed = schlafli{4, 3, 3}:vec('w'),
    depths = depths,
    letters = {'R', 'U', 'L', 'D', 'F', 'B', 'O', 'I'},
    order = {'R', 'L', 'U', 'D', 'F', 'B', 'O', 'I'},
  }
end
