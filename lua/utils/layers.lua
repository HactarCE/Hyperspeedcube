-- Functions for generating layer cut tables



-- Returns evenly-spaced layer depths, including both endpoints
--
-- Typically `start > stop`
function inclusive(start, stop, layers)
  if layers < 1 then
    error(string.format(
      "cannot build 0 layers in the range from %d to %d inclusive",
      start,
      stop
    ))
  end

  local ret = {}
  for i = 0, layers do
    ret[i + 1] = start + (stop - start) * i / layers
  end
  return ret
end

-- Returns evenly-spaced layer depths for half of a puzzle
--
-- For even numbers of layers, includes both endpoints
-- For odd numbers of layers, includes `start` but not `stop`
--
-- Expects `start > stop`
function even_odd(start, stop, layers)
  local half_layer_size = (stop - start) / layers
  if layers % 2 == 1 then
    stop = stop - half_layer_size
  end

  return inclusive(start, stop, floor(layers/2))
end
