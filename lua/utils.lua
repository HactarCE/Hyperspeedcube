function twist3d(axis, twist_transform)
  return {
    axis = axis,
    transform = twist_transform,
    prefix = axis.name,
    inverse = true,
    multipliers = true,
  }
end

function map_string_keys(t, map)
  if type(t) == 'string' then
    if type(map) == 'function' then
      return map(t) or t
    else
      return map[t] or t
    end
  elseif type(t) == 'table' then
    local ret = {}
    for k, v in pairs(t) do
      ret[map_string_keys(k,map)] = v
    end
    return ret
  else
    return t
  end
end

function map_string_values(t, map)
  if type(t) == 'string' then
    if type(map) == 'function' then
      return map(t) or t
    else
      return map[t] or t
    end
  elseif type(t) == 'table' then
    local ret = {}
    for k, v in pairs(t) do
      ret[k] = map_string_values(v, map)
    end
    return ret
  else
    return t
  end
end

-- Returns evenly-spaced layer depths, excluding both endpoints
function layers_exclusive(start, stop, steps)
  local ret = {}
  for i = 1, steps do
    ret[i] = i / (steps + 1) * (stop - start) + start
  end
  return ret
end

function double_ended_layers(start, stop, steps)
  local ret = {}
  for i = 1, steps do
    ret[i] = i / steps * (stop - start) + start
  end
  return ret
end

-- Returns evenly-spaced layer depths, including both endpoints
function layers_inclusive(start, stop, steps)
  local ret = {}
  for i = 1, steps do
    if math.eq(start, stop) then
      ret[i] = start
    elseif steps <= 1 then
      error(string.format(
        "cannot build 1 layer in the range from %d to %d inclusive",
        start,
        stop
      ))
    else
      ret[i] = (i - 1) / (steps - 1) * (stop - start) + start
    end
  end
  return ret
end

-- Returns evenly-spaced layer depths for half of a puzzle
function even_odd_layers(start, stop, layers)
  local ret = {}

  local half_layer_size = (stop - start) / layers
  start = start + 2 * half_layer_size
  if layers % 2 == 1 then
    stop = stop - half_layer_size
  end

  return layers_inclusive(start, stop, floor(layers/2))
end
