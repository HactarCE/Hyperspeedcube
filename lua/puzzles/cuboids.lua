CORNER_STALK_SIZE = 1 -- measured in terms of the height of an inner layer

local function canonicalize_cuboid_params(params)
  for i = 1, #params-1 do
    if params[i] > params[i+1] then
      table.sort(params)
      return params
    end
  end
end

local function cuboid_name(sizes)
  local ret = tostring(sizes[1])
  for i = 2, #sizes do
    ret = ret .. "x" .. sizes[i]
  end
  return ret
end

-- Carves a cuboid and returns the inner layer height
local function carve_cuboid(puzzle, sizes)
  -- Subtract 2 from each size
  local inner_sizes = {}
  for i, size in ipairs(sizes) do
    inner_sizes[i] = size - 2
  end

  local squared_corner_distance = vec(inner_sizes).mag2

  local excesses = {}
  for i, inner_size in ipairs(inner_sizes) do
    -- Find the minimum inner size on any axis other than this one
    local min_other_inner_size = INF
    for j, other_inner_size in ipairs(inner_sizes) do
      if i ~= j and other_inner_size < min_other_inner_size then
        min_other_inner_size = other_inner_size
      end
    end

    excesses[i] = sqrt(squared_corner_distance - min_other_inner_size^2) - inner_size
  end

  -- The excess must be at least 1 (the height of an inner layer)
  local excess = math.max(1, table.unpack(excesses)) + CORNER_STALK_SIZE

  local half_layer_height = 1 / (math.max(table.unpack(inner_sizes)) + excess)
  print(half_layer_height)

  for i, inner_size in ipairs(inner_sizes) do
    local half_height = half_layer_height * (inner_size + excess)
    puzzle:carve(vec{[i] = half_height})
    puzzle:carve(vec{[i] = -half_height})
  end

  return half_layer_height * 2
end

local function slice_cuboid(puzzle, sizes, layer_height)
  for i, size in ipairs(sizes) do
    local d = layer_height * (size - 2) / 2
    local cut_depths = lib.utils.layers.inclusive_inf(d, -d, size)
    local v = vec{[i] = 1}
    -- puzzle.axes[v].layers:add(cut_depths)
    -- puzzle.axes[-v].layers:add(cut_depths)
    for _, depth in ipairs(cut_depths) do
      puzzle:slice(plane(v, depth))
    end
  end
end

local function cuboid_twist_transform(sizes, axis1, axis2)
  local angle
  if sizes[axis1] % 2 == sizes[axis2] % 2 then
    angle = pi/2
  else
    angle = pi
  end
  return rot{from = vec{[axis1] = 1}, to = vec{[axis2] = 1}, angle = angle}
end

local function cuboid_layers(layer_height, layer_count)
  local d = layer_height * (layer_count-2) / 2
  return lib.utils.layers.inclusive_inf(d, -d, layer_count)
end

puzzle_generators:add{
  id = 'cuboid',
  version = '0.1.0',
  name = "AxBxC Cuboid",
  colors = 'cube',
  params = {
    { name = "A", type = 'int', default = 3, min = 1, max = 49 },
    { name = "B", type = 'int', default = 4, min = 1, max = 49 },
    { name = "C", type = 'int', default = 5, min = 1, max = 49 },
  },
  gen = function(params)
    -- Redirect to cube if possible
    if params[1] == params[2] and params[2] == params[3] then
      return 'ft_cube', {params[1]}
    end

    -- Canonicalize parameter order
    local new_params = canonicalize_cuboid_params(params)
    if new_params then
      return 'cuboid', params
    end
    local sizes = params

    return {
      name = cuboid_name(sizes) .. " Cuboid",
      ndim = 3,
      build = function(self)
        local inner_layer_height = carve_cuboid(self, sizes)
        self.axes:add(lib.symmetries.cubic.cube():iter_poles())
        -- Add layers
        slice_cuboid(self, sizes, inner_layer_height)

        lib.utils.unpack_named(_ENV, self.axes)

        local max_even_size = 0
        local max_odd_size = 0
        for _, size in ipairs(sizes) do
          if size % 2 == 0 and size > max_even_size then max_even_size = size end
          if size % 2 == 1 and size > max_odd_size then max_odd_size = size end
        end

        for i, size in ipairs(sizes) do
          local v = vec{[i] = 1}
          local cut_depths
          if size % 2 == 0 then
            cut_depths = cuboid_layers(inner_layer_height, max_even_size)
          else
            cut_depths = cuboid_layers(inner_layer_height, max_odd_size)
          end
          self.axes[v].layers:add(cut_depths)
          self.axes[-v].layers:add(cut_depths)
        end

        local sym = cd{2, 2, 2}
        local function add_twist_pair(i, j, k)
          local v = vec{[i] = 1}
          for _, ax, transform in sym.chiral:orbit(self.axes[v], cuboid_twist_transform(sizes, j, k)) do
            self.twists:add(ax, transform, { gizmo_pole_distance = 1 })
          end
        end
        add_twist_pair(1, 3, 2)
        add_twist_pair(2, 1, 3)
        add_twist_pair(3, 2, 1)
      end,
    }
  end,
}

puzzle_generators:add{
  id = 'hypercuboid',
  version = '0.1.0',
  name = "AxBxCxD Cuboid",
  colors = 'hypercube',
  params = {
    { name = "A", type = 'int', default = 3, min = 1, max = 49 },
    { name = "B", type = 'int', default = 4, min = 1, max = 49 },
    { name = "C", type = 'int', default = 5, min = 1, max = 49 },
    { name = "D", type = 'int', default = 6, min = 1, max = 49 },
  },
  gen = function(params)
    -- Canonicalize parameter order
    local new_params = canonicalize_cuboid_params(params)
    if new_params then
      return 'hypercuboid', params
    end
    local sizes = params

    return {
      name = cuboid_name(sizes) .. " Cuboid",
      ndim = 4,
      build = function(self)
        local shape = lib.symmetries.hypercubic.hypercube()
        lib.utils.cut_ft_shape(self, shape)
        self.axes:add(shape:iter_poles())

        -- Add layers
        slice_cuboid(self, sizes)
      end,
    }
  end,
}
