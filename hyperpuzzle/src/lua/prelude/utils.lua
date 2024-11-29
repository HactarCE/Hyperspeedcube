-- Remove randomness
math.random = nil
math.randomseed = nil
-- Remove ambiguous degree/radian functions
math.deg = nil
math.rad = nil
-- Add extra constants
math.tau = math.pi * 2
math.degree = math.pi / 180
math.phi = (1 + math.sqrt(5)) / 2

function string.fmt2(s1, s2, ...)
  return string.format(s1, ...), string.format(s2, ...)
end

-- for use from Rust code
function builtin_concat(a, b)
  return a .. b
end
