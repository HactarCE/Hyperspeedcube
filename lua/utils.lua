for k, v in pairs(require('utils/*')) do
  _G[k] = v
end

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
    ret = ret * 26 + string.byte(string.sub(name, i, i)) - string.byte('A') + 1
  end
  return ret
end

function cut_shape(puzzle, shape, cut_depths, ...)
  local poles = shape:iter_poles(...)
  local colors = puzzle:carve(poles)
  local axes = cut_depths and puzzle.axes:add(poles, cut_depths)
  return colors, axes
end
