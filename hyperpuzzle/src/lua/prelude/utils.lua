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
