function twist3d(axis, transform)
  return {
    axis = axis,
    transform = transform,
    prefix = axis.name,
    inverse = true,
    multipliers = true,
  }
end
