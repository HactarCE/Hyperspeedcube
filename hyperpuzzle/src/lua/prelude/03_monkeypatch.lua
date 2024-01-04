function string.join(connector, t)
  connector = connector or ', '
  local result = ''
  for i, v in ipairs(t) do
    if i > 1 then
      result = result .. connector
    end
    result = result .. tostring(v)
  end
  return result
end

function collect(...)
  if type(...) == 'table' then
    return ...
  end

  local t = {}
  for elem in ... do
    table.insert(t, elem)
  end
  return t
end

function iter(t)
  if type(t) == 'table' then
    local i = 0
    local n = #t
    return function()
      i = i + 1
      if i <= n then return t[i] end
    end
  else
    return t
  end
end

-- Remove randomness
math.random = nil
math.randomseed = nil
-- Add extra constants
math.tau = math.pi * 2
math.phi = (1 + math.sqrt(5)) / 2
