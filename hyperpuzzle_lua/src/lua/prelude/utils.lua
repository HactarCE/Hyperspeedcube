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

-- Add hyperbolic trig
local exp = math.exp
local log = math.log
local sqrt = math.sqrt
math.sinh = function(x) return 0.5 * (exp(x) - exp(-x)) end
math.cosh = function(x) return 0.5 * (exp(x) + exp(-x)) end
math.tanh = function(x) return (exp(2*x) - 1) / (exp(2*x) + 1) end
math.asinh = function(x) return log(x + sqrt(x*x + 1)) end
math.acosh = function(x) return log(x + sqrt(x*x - 1)) end
math.atanh = function(x) return 0.5 * log((1+x)/(1-x)) end

function string.fmt2(s1, s2, ...)
  return string.format(s1, ...), string.format(s2, ...)
end

-- for use from Rust code
function builtin_concat(a, b)
  return a .. b
end
