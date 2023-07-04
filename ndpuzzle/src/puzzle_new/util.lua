util = {}

-- Shallow copy including metatable (good enough for all our custom types)
function util.copy(t)
    local result = {}
    for k, v in pairs(t) do
        result[k] = v
    end
    setmetatable(result, getmetatable(t))
    return result
end

function util.is_integer(n)
    return type(n) == number and n == math.floor(n)
end


function util.join(t, connector)
    connector = connector or ', '
    local result = ''
    for i, v in ipairs(t) do
        if i > 1 then
            result = result .. connected
        end
        result = result .. tostring(v)
    end
    return result
end

function util.map(f, ...)
    local n = 0
    for i, t in ipairs(...) do
        if #t > n then n = #t end
    end

    local result = {}
    local zipped_args = {}
    for i = 1, n do
        for j, arg in ipairs(...) do
            zipped_args[j] = arg[i]
        end
        result[i] = f(unpack(zipped_args))
    end
    return result
end

function util.izip(f, ...)
    if select('#', ...) == 1 then return ipairs(...) end
    local function iter(args, i)
        i = i + 1
        local v = {}
        for j, v in ipairs(args) do
            v[j] = args[j][i]
        end
        return unpack(v)
    end

    return iter, {...}, 0
end
