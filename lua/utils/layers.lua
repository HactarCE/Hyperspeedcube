-- Functions for generating layer cut tables



-- Returns evenly-spaced layer depths, excluding both endpoints
--
-- Typically `start > stop`
function exclusive(start, stop, steps)
  local ret = {}
  for i = 1, steps do
    ret[i] = i / (steps + 1) * (stop - start) + start
  end
  return ret
end

-- Returns evenly-spaced layer depths, excluding `start` but including `end`
--
-- Typically `start > stop`
function double_ended(start, stop, steps)
  local ret = {}
  for i = 1, steps do
    ret[i] = i / steps * (stop - start) + start
  end
  return ret
end

-- Returns evenly-spaced layer depths, including both endpoints
--
-- Typically `start > stop`
function inclusive(start, stop, steps)
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
--
-- Expects `start > stop`
function even_odd(start, stop, layers)
  local ret = {}

  local half_layer_size = (stop - start) / layers
  start = start + 2 * half_layer_size
  if layers % 2 == 1 then
    stop = stop - half_layer_size
  end

  return inclusive(start, stop, floor(layers/2))
end
