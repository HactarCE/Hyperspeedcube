function lerp(a, b, t)
  return a + (b-a)*t
end

function nth_uppercase_name(n)
  local ret = ''
  while n > 0 do
    n = n - 1
    ret = string.char(string.byte('A') + (n%26)) .. ret
    n = floor(n / 26)
  end
  return ret
end

function uppercase_name_to_n(name)
  local ret = 0
  for i = 1, #name do
    ret = ret * 26 + string.byte(name:sub(i, i)) - string.byte('A') + 1
  end
  return ret
end

function cut_ft_shape(puzzle, shape, cut_depths, ...)
  local poles = shape:iter_poles(...)
  local colors = puzzle:carve(poles)
  local axes = cut_depths and puzzle.axes:add(poles, cut_depths)
  return colors, axes
end

-- Concatenates the sequences.
function concatseq(...)
  local ret = {}
  for _, t in ipairs({...}) do
    for _, v in ipairs(t) do
      table.insert(ret, v)
    end
  end
  return ret
end

-- Functions for generating layer cut tables
layers = {}

-- Returns evenly-spaced layer depths, including both endpoints
--
-- Typically `start > stop`
function layers.inclusive(start, stop, layer_count)
  if layer_count < 1 then
    return nil
  end

  local ret = {}
  for i = 0, layer_count do
    ret[i + 1] = start + (stop - start) * i / layer_count
  end
  return ret
end

-- Returns evenly-spaced layer depths for half of a puzzle
--
-- For even numbers of layers, includes both endpoints
-- For odd numbers of layers, includes `start` but not `stop`
--
-- Expects `start > stop`
function layers.even_odd(start, stop, layer_count)
  local half_layer_size = (stop - start) / layer_count
  if layer_count % 2 == 1 then
    stop = stop - half_layer_size
  end

  return layers.inclusive(start, stop, floor(layer_count/2))
end

-- Returns evenly-spaced layer depths, including both endpoints and with `INF`
-- and `-INF` on either side.
function layers.inclusive_inf(start, stop, layer_count)
  if layer_count < 1 then
    return nil
  elseif layer_count == 1 then
    return {INF, -INF}
  elseif layer_count == 2 then
    return {INF, (start+stop)/2, -INF}
  else
    return concatseq({INF}, layers.inclusive(start, stop, layer_count-2), {-INF})
  end
end

-- Returns evenly-spaced layer depths, excluding both endpoints and with `INF`
-- on one side.
function layers.exclusive_centered(center, half_range, cut_count)
  if cut_count == 0 then
    return {}
  elseif cut_count == 1 then
    return {center}
  else
    local half_layer_height = half_range / (cut_count + 1)
    local outermost_cut = center + half_range - half_layer_height
    local innermost_cut = center - half_range + half_layer_height
    return layers.inclusive(outermost_cut, innermost_cut, cut_count-1)
  end
end

function unpack_named(env, elements)
  for k,v in ipairs(elements) do
    env[tostring(v.name)] = v
  end
end
