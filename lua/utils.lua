function lerp(a, b, t)
  return a + (b-a)*t
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
  for i = #name, 1, -1 do
    string.sub(name, i, 1)
  end
end
