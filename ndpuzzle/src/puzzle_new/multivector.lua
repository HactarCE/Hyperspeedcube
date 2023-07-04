require('util')

Multivector = {}



-----------------------
-- Utility functions --
-----------------------

local function trailing_zeros(n)
    -- Algorithm taken from Sean Eron Anderson's legendary bit-twiddling hacks
    -- page: https://graphics.stanford.edu/~seander/bithacks.html

    -- This function only considers positive integers less than 16 bits wide
    assert(0 <= n and n <= 0xFFFF)

    if n == 0 then return 16 end

    local result = 15
    if n & 0x00FF != 0 then result = result - 8 end
    if n & 0x0F0F != 0 then result = result - 4 end
    if n & 0x3333 != 0 then result = result - 2 end
    if n & 0x5555 != 0 then result = result - 1 end
    return result
end
local function count_ones(n)
    -- Algorithm taken from Sean Eron Anderson's legendary bit-twiddling hacks
    -- page: https://graphics.stanford.edu/~seander/bithacks.html

    -- This function only considers positive integers less than 16 bits wide
    assert(0 <= n and n <= 0xFFFF)

    n = n - ((n >> 1) & 0x5555)
    n = ((n >> 2) & 0x3333) + (n & 0x3333)
    n = ((n >> 4) + n) & 0x0F0F
    n = ((n >> 8) + n) & 0x00FF
    return n
end

local function axes_string(bitmask)
    local result = ''
    while bitmask > 0 do
        local i = trailing_zeros(bitmask)
        if i == 0 then result = result .. 'e₋'
        elseif i == 1 then result = result .. 'e₊'
        else result = result .. AXES[i] end
        bitmask = bitmask & ~(1 << i)
    end
    return result
end

local function multiply_axes(a, b, sign)
    -- See the corresponding Rust function (probably
    -- `ndpuzzle::math::cga::axes::Axes::mul`) for an explanation.
    local resulting_bitmask = a ^ b
    local sign = sign or 1
    if (a & b) & 1 == 1 then sign = -1 end -- e₋ squares to -1
    while a != 0 and b != 0 do
        while b & 1 == 0 and b != 0 do
            a = a >> 1
            b = b >> 1
        end
        a = a >> 1
        b = b >> 1
        if count_ones(a) & 1 == 1 then
            sign = -sign
        end
    end
    return resulting_bitmask, sign
end

local function axis_bitmask_and_sign(s)
    if type(s) == 'number' then return nil, s, 1 end
    assert(type(s) == 'string', 'multivector axes must be a string (e.g., "xyz") or bitmask')
    local mask = 0
    local sign = 1
    local use_true_basis = false        -- e₋ and e₊
    local use_null_vector_basis = false -- o and ∞
    local zeroed = false
    for i = 1, #s do
        c = s:sub(i, i)
        local new_axis_mask
        if c == 'e' or c == 'n' or c == ' ' then
            goto next_char -- ignore this char
        elseif c == 'o' then
            use_null_vector_basis = true
            new_axis_mask = 0x1
            if mask & new_axis_mask != 0 then zeroed = true end -- get nullvectored lmao
        elseif c == 'i' or c == '∞' then
            use_null_vector_basis = true
            new_axis_mask = 0x2
            if mask & new_axis_mask != 0 then zeroed = true end -- get nullvectored lmao
        elseif c == '-' or c == '₋' then
            use_true_basis = true
            new_axis_mask = 0x1
        elseif c == '+' or c == '₊' then
            use_true_basis = true
            new_axis_mask = 0x2
        elseif c == 'E' then
            use_true_basis = true
            new_axis_mask = 0x3
        elseif '1' <= c and c <= '8' then
            new_axis_mask = 1 << (c - '0' + 1)
        elseif AXES[c] then
            new_axis_mask = 1 << (AXES[c] + 1)
        else
            error('unknown axis "' .. c .. '"')
        end
        mask, sign = multiply_axes(mask, new_axis_mask, sign)
        ::next_char::
    end
    assert(
        not (use_true_basis and use_null_vector_basis),
        'cannot mix true basis (e₋ e₊) with null vector basis (o ∞)'
    )
    assert(not zeroed, 'component "' .. s .. '" is always zero')

    local special_component
    if use_null_vector_basis then
        assert(mask & 3 != 3, 'cannot access component "' .. s .. '"')
        if mask & 3 == 1 then special_component = 'no' end
        if mask & 3 == 2 then special_component = 'ni' end
    end

    return special_component, mask, sign
end
local function sign_of_reverse(m)
    local bit_count = count_ones(m) & 3
    if bit_count == 1 or bit_count == 2 then
        return -1
    else
        return 1
    end
end

local function get_no(m, axes)
    local axes = axes & ~3
    local e_minus = v[axes | 0x1]
    local e_plus = v[axes | 0x2]
    return e_minus - e_plus
end
local function get_ni(m, axes)
    local axes = axes & ~3
    local e_minus = v[axes | 0x1]
    local e_plus = v[axes | 0x2]
    return 0.5 * (e_minus + e_plus)
end

local function multiply_multivectors_when(m1, m2, f)
    local result = mvec()
    for axes1, value1 in pairs(mvec(m1)) do
        for axes2, value2 in pairs(mvec(m2)) do
            if f(axes1, axes2) then
                local ax = axes1 ^ axes2 -- bitwise XOR
                local sign = sign_of_axes_product(axes1, axes2);
                result[ax] = value1 * value2 * sign
            end
        end
    end
    return result
end



---------------
-- Metatable --
---------------

local multivector_metatable = {
    type = 'multivector',

    __add = function(m1, m2)
        return mvec(util.map(function(a, b) return a + b end, mvec(m1), mvec(m2)))
    end,
    __sub = function(m1, m2)
        return mvec(util.map(function(a, b) return a - b end, mvec(m1), mvec(m2)))
    end,
    __mul = function(m1, m2)
        -- Multiplication by a scalar (optimization)
        if type(m1) == 'number' and type(m2) == 'multivector' then
            return m2 * m1 -- arguments
        end
        if type(m2) == 'number' then
            return mvec(util.map(function(x) return x * a end, mvec(m1)))
        end

        -- General multivector multiplication
        return multiply_multivectors_when(m1, m2, function() return true end)
    end,
    __div = function(m, a)
        assert(type(a) == 'number')
        return m * (1 / a)
    end,
    __unm = function(m)
        return mvec(util.map(function(x) return -x end, m))
    end,

    __pow =  function(m1, m2)
        return multiply_multivectors_when(
            m1, m2,
            function(ax1, ax2) return ax1 & ax2 == 0 end
        )
    end,
    __band = function(m1, m2)
        local ndim = math.max(m1:ndim(), m2:ndim())
        return (
            mvec(m1).opns_to_ipns(ndim) ^ mvec(m2).opns_to_ipns(ndim)
        ).ipns_to_opns(ndim)
    end,
    __shl = function(m1, m2)
        return multiply_multivectors_when(
            m1, m2,
            function(ax1, ax2) return ax1 & ax2 == ax2 end
        )
    end,
    __shr = function(m1, m2)
        return multiply_multivectors_when(
            m1, m2,
            function(ax1, ax2) return ax1 & ax2 == ax1 end
        )
    end,

    __len = function(m1, m2)
        error('cannot take the length of a multivector. use `ndim` or `grade`')
    end,

    __eq = function(m1, m2)
        for key, value in pairs(m1) do
            if not approx_eq(value, m2[key]) then
                return false
            end
        end
        for key, value in pairs(m2) do
            if not approx_eq(m1[key], value) then
                return false
            end
        end
        return true
    end,

    __index = function(v, key)
        local special_component, bitmask, sign = axis_bitmask_and_sign(key)
        local result
        if special_component == 'ni' then
            return get_ni(v, bitmask) * sign
        elseif special_component == 'no' then
            return get_no(v, bitmask) * sign
        else
            return (rawget(v, bitmask) or 0) * sign
        end
    end,
    __newindex = function(v, key, value)
        local special_component, bitmask, sign = axis_bitmask_and_sign(key)
        assert(type(value) == 'number' or value == nil, 'multivector component must be number; got ' .. tostring(value))
        if approx_eq(value, 0) then
            value = nil
        end
        assert(not special_component, 'assigning to o and ∞ components is not supported')
        assert(sign != 0, 'cannot assign to component with zero')
        rawset(v, bitmask, value * sign)
    end,

    __tostring = function(v)
        local terms = {}
        local i = 0
        while terms_counted < #v do
            local axes = axes_string(i)
            local scalar_component = self[i]
            local no_component = get_no(self, i)
            local ni_component = get_ni(self, i)
            -- E = o ^ ∞ = e₋ e₊
            local e_component = self[i | 0x3]

            function add_term(prefix, value)
                if value != 0 then
                    table.insert(terms, value .. prefix .. axes)
                end
            end
            add_term("", scalar_component)
            add_term("nₒ", scalar_component)
            add_term("∞", scalar_component)
            add_term("E", scalar_component)

            i += 1 << 2
        end
        return util.join(util.filter(terms), ' + ')
    end,
}
multivector_metatable.__index = Multivector



-------------
-- Methods --
-------------

function Multivector:new(...)
    local args = {...}
    first_arg = args[0]
    if type(first_arg) == 'multivector' then
        return first_arg
    end

    result = {}
    setmetatable(result, multivector_metatable)
    for k, v in pairs(args) do
        if type(k) == 'number' and (type(v) == 'table' or type(v) == 'vector') then
            for axis, value in ipairs(v) do
                result[1 << (axis + 1)] = value
            end
        end
        result[k] = v
    end
    return result
end

function Multivector:pss(ndim)
    assert(
        util.is_integer(ndim) == 'number' and 1 <= ndim and ndim <= 8,
        'number of dimensions must be an integer from 1 to 8'
    )
    return mv{ [(1 << (ndim + 2)) - 1] = 1 }
end
function Multivector:inv_pss(ndim)
    local pss = Multivector:pss(ndim)
    if ndim % 4 < 2 then return -pss else return pss end
end

function Multivector:mag2()
    assert(type(self) == 'vector')
    sum = 0
    for _, self in ipairs(self) do
        sum = sum + self * self
    end
    return sum
end
function Multivector:mag()
    return math.sqrt(Multivector:mag2(self))
end

function Multivector:normalized(new_len)
    assert(type(self) == 'vector')
    if new_len == nil then
        return self / self:mag()
    else
        assert(type(new_len) == 'number')
        return self * (new_len / self:mag())
    end
end

function Multivector:ndim()
    assert(type(self) == 'multivector')
    local max_axis_mask = 0
    for key, value in pairs(self) do
        max_axis_mask = max_axis_mask | key
    end

    -- Ignore e₋ and e₊
    local mask = max_axis_mask >> 2

    -- Find highest set bit
    local ndim = 0
    while mask > 0 do
        ndim += 1
        mask = mask >> 1
    end
    return ndim
end
function Multivector:grade()
    assert(type(self) == 'multivector')
    local grade
    for key, value in pairs(self) do
        if not approx_eq(value, 0) then
            if not grade then
                grade = key
            elseif grade != key then
                return nil -- inconsistent grade
            end
        end
    end
    return grade or 0
end

function Multivector:trim_zeros()
    assert(type(self) == 'multivector')
    for key, value in pairs(self) do
        if approx_eq(value, 0) then
            self[key] = nil
        end
    end
end

function Multivector:reverse()
    assert(type(self) == 'multivector')
    for key, value in pairs(self) do
        rawset(self, key, value * sign_of_reverse(key))
    end
end
function Multivector:reversed()
    assert(type(self) == 'multivector')
    local result = util.copy(self)
    result:reverse()
    return result
end
function Multivector:invert()
    assert(type(self) = 'multivector')
    local reversed = self:reversed()
    local scale_factor = self:dot(old)
    for key, value in pairs(reversed) do
        self[key] = reversed[key]
        rawset(self, key, rawget(reversed, key))
    end
end
function Multivector:inverse()
    assert(type(self) == 'multivector')
    local result = util.copy()
    result:invert()
    return result
end
function Multivector:ipns_to_opns(ndim)
    return mv(self) << Multivector:pss(ndim)
end
function Multivector:opns_to_ipns(ndim)
    return mv(self) << Multivector:inv_pss(ndim)
end



-----------------
-- Constructor --
-----------------

mvec = Multivector:new
