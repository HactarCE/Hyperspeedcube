for k, v in pairs(require('symmetries/*')) do
  print(k)
  _G[k] = v
end
