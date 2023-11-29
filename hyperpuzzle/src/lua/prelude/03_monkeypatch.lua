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

-- Remove randomness
math.random = nil
math.randomseed = nil
-- Add extra constants
math.tau = math.pi * 2
math.phi = (1 + math.sqrt(5)) / 2
