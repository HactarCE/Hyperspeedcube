import itertools

NDIM = 3

AXES = "mpxyzwuv"


def components(grade):
    return map(''.join, itertools.combinations(AXES[:NDIM+2], grade))


def prop(components):
    return components or 's'


def gen_blade_struct(r):
    ret = ''
    ret += f'struct Blade{r} {{'
    for component in components(r):
        ret += f' float {prop(component)};'
    ret += f' }};'
    return ret


def gen_wedge_fn(r, s):
    return gen_mul_func('wedge', r, s, r+s)


def gen_left_contract_fn(r, s):
    return gen_mul_func('lc', r, s, s-r)


def gen_mul_func(funcname, r, s, result_grade):
    if result_grade <= 0:
        return ''
    ret = ''
    ret += f'Blade{result_grade} {funcname}_{r}_{s}(Blade{r} a, Blade{s} b) {{\n'

    # construct return value
    ret += f'  Blade{result_grade} ret;\n'

    # compute return value
    for a in components(r):
        for b in components(s):
            ab, sign = multiply_axes(a, b)
            sign = '-' if sign < 0 else ' '
            if len(ab) == result_grade:
                ret += f'  ret.{prop(ab)} += {sign}a.{prop(a)} * b.{prop(b)};\n'

    # return return value
    ret += f'  return ret;\n'

    ret += f'}}\n'
    return ret


def multiply_axes(a, b):
    ret = ''
    sign = 1
    for ax in AXES:
        if not a or not b:
            return ret + a + b, sign
        if a[0] == ax:
            ret += ax
            a = a[1:]
        if b[0] == ax:
            ret += ax
            b = b[1:]
            sign *= (-1) ** len(a)
        if ret.endswith(ax + ax):
            ret = ret[:-2]
            if ax == 'm':
                sign *= -1
    if a or b:
        raise Exception(f'bad axes: {a!r} and {b!r}')
    return ret, sign


for i in range(NDIM+3):
    print(gen_blade_struct(i))
print()
print()
for i in range(1, NDIM+2):
    for j in range(1, NDIM+2):
        print(gen_wedge_fn(i, j))
        print(gen_left_contract_fn(i, j))
