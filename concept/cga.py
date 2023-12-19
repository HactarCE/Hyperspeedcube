from collections import defaultdict, Counter
from typing import Iterable, Optional
import itertools

AXES = '-+xyz'
GANJA_AXES = {
    '-': 'eminus',
    '+': 'eplus',
    'i': 'ni',
    'o': 'no',
    'x': '1e1',
    'y': '1e2',
    'z': '1e3',
    'E': '(eminus*eplus)'
}

LATEX = False


def powerset(iterable):
    "powerset([1,2,3]) --> () (1,) (2,) (3,) (1,2) (1,3) (2,3) (1,2,3)"
    s = list(iterable)
    return itertools.chain.from_iterable(itertools.combinations(s, r) for r in range(len(s)+1))


def move_to_front(s, c):
    i = s.index(c)
    return (c + s[:i] + s[i+1:], i)


def canonicalize(axes: str) -> tuple[str, float]:
    ret = ''
    swaps = 0
    axes = ''.join(axes)
    for c in AXES:
        while c in axes:
            swaps += axes.index(c)
            axes = axes.replace(c, '', 1)
            ret += c
    swaps += ret.count('--')
    for c in AXES:
        ret = ret.replace(c+c, '')
    sign = -1 if swaps % 2 else 1
    return (ret, sign)


class ScalarExpr:
    def __init__(self, terms: Iterable[tuple[tuple, float]] = None):
        if isinstance(terms, float) or isinstance(terms, int):
            terms = [((), terms)]
        elif isinstance(terms, str):
            terms = [((terms,), 1)]
        elif isinstance(terms, dict):
            terms = terms.items()
        self.terms = defaultdict(lambda: 0)
        if terms is not None:
            for k, v in terms:
                if v != 0:
                    self.terms[tuple(sorted(k))] += v

    def __mul__(self, rhs):
        if isinstance(rhs, ScalarExpr):
            new_terms = defaultdict(lambda: 0)
            for vars1, coef1 in self.terms.items():
                for vars2, coef2 in rhs.terms.items():
                    vars = vars1 + vars2
                    coef = coef1 * coef2
                    new_terms[vars] += coef
            return ScalarExpr(new_terms)
        else:
            return ScalarExpr({k: v * rhs for k, v in self.terms.items()})

    def __div__(self, rhs):
        return self * (1/rhs)

    def __pow__(self, rhs):
        assert rhs == 2
        return self * self

    def __add__(self, rhs):
        if isinstance(rhs, ScalarExpr):
            new_terms = self.terms.copy()
            for (vars, coef) in rhs.terms.items():
                new_terms[vars] += coef
            return ScalarExpr({k: v for k, v in new_terms.items() if v != 0})
        else:
            return self + ScalarExpr([('', rhs)])

    def __sub__(self, rhs):
        return self + -rhs

    def __neg__(self):
        return self * -1

    def __bool__(self):
        return any(self.terms.values())

    def __str__(self):
        def var_to_string(var, power):
            if power == 1:
                return var
            else:
                if LATEX:
                    return f'{{{var}}}^{power}'
                else:
                    return f'{var}**{power}'

        def term_to_string(coef, vars):
            vars = Counter(vars)
            if LATEX:
                var_string = ' '.join(var_to_string(v, vars[v])
                                      for v in sorted(vars.keys()))
            else:
                var_string = ' * '.join(var_to_string(v, vars[v])
                                        for v in sorted(vars.keys()))
            if coef == 1 and var_string:
                return var_string
            elif coef == -1 and var_string:
                return f'-{var_string}' if LATEX else f'-({var_string})'
            else:
                return f'{coef} {var_string}' if LATEX else f'{coef} * {var_string}'

        return ' + '.join(
            term_to_string(self.terms[var], var)
            for var in sorted(self.terms.keys())
        )

    def __eq__(self, scalar):
        return len(self.terms) == 1 and () in self.terms and self.terms[()] == scalar

    @property
    def recip(self):
        keys = list(self.terms.keys())
        if keys == []:
            raise Exception("cannot invert 0")
        elif keys == [()]:
            return ScalarExpr(1/self.terms[()])
        else:
            raise Exception(f"cannot invert {self}")


class CgaExpr:
    def __init__(self, terms: Optional[Iterable[tuple[str, ScalarExpr]]] = None):
        if isinstance(terms, ScalarExpr) or isinstance(terms, float) or isinstance(terms, int):
            terms = [((), terms)]
        elif isinstance(terms, str):
            terms = [((terms,), 1)]
        elif isinstance(terms, dict):
            terms = terms.items()
        self.terms = defaultdict(ScalarExpr)
        if terms is not None:
            for k, v in terms:
                if v:
                    k, sign = canonicalize(k)
                    self.terms[k] += v * sign

    def __mul__(self, rhs):
        if isinstance(rhs, CgaExpr):
            new_terms = defaultdict(ScalarExpr)
            for axes1, coef1 in self.terms.items():
                for axes2, coef2 in rhs.terms.items():
                    axes, sign = canonicalize(axes1 + axes2)
                    coef = coef1 * coef2 * sign
                    new_terms[axes] += coef
            return CgaExpr(new_terms)
        else:
            return CgaExpr({k: v * rhs for k, v in self.terms.items()})

    def __rmul__(self, other):
        return CgaExpr(other) * self

    def __div__(self, rhs):
        return self * (1/rhs)

    def __pow__(self, rhs):
        assert rhs == 2
        return self * self

    def __neg__(self):
        return self * -1

    def __add__(self, rhs):
        if not isinstance(rhs, CgaExpr):
            rhs = CgaExpr(rhs)
        new_terms = self.terms.copy()
        for (vars, coef) in rhs.terms.items():
            new_terms[vars] += coef
        return CgaExpr(new_terms)

    def __sub__(self, rhs):
        return self + -rhs

    def __lshift__(self, rhs):
        return (self * rhs).grade_project(rhs.grade - self.grade)

    def __xor__(self, rhs):
        return (self * rhs).grade_project(self.grade + rhs.grade)

    def __bool__(self):
        return any(self.terms.values())

    def __str__(self):
        if LATEX:
            return '&= ' + '\\\\\n&+ '.join(
                it
                for axes in map(lambda x: ''.join(x), powerset(AXES[2:]))
                for it in [
                    self.get_human_term(axes),
                    self.get_human_term('o'+axes),
                    self.get_human_term('i'+axes),
                    self.get_human_term('E'+axes),
                ]
                if it
            ).replace('.0 ', ' ').replace(' + -', ' - ')
        else:
            return ' + '.join(
                self.get_human_term(axes)
                for axes in map(lambda x: ''.join(x), powerset(AXES))
                if self.get_human_term(axes)
            ).replace('.0 ', ' ').replace(' + -', ' - ')

    def get_human_term(self, axes):
        a = axes[1:]
        if axes.startswith('o'):
            coef = self.terms[f'-{a}'] - self.terms[f'+{a}']
        elif axes.startswith('i'):
            coef = (self.terms[f'-{a}'] + self.terms[f'+{a}']) * 0.5
        elif axes.startswith('E'):
            coef = self.terms[f'-+{a}']
        else:
            coef = self.terms[axes]

        if not coef:
            return ''

        if LATEX:
            axes_string = axes.replace('i', '\infty ')
        else:
            axes_string = '*'.join(GANJA_AXES[c] for c in axes)
        if coef == 1 and axes_string:
            return axes_string
        elif coef == -1 and axes_string:
            return f'-{axes_string}'
        elif len(coef.terms.items()) > 1:
            if LATEX:
                return f'({coef}) {axes_string}'
            else:
                return f'({coef}) * {axes_string}'
        else:
            if LATEX:
                return f'{coef} {axes_string}'
            else:
                return f'{coef} * {axes_string}'

    @property
    def grade(self):
        return len(next(iter(self.terms.keys())))

    @property
    def s(self):
        return self.terms.get('', ScalarExpr(0))

    @property
    def rev(self):
        return CgaExpr({k[::-1]: v for k, v in self.terms.items()})

    @property
    def inv(self):
        # Formula from https://math.stackexchange.com/a/556232/1115019
        return self.rev * (self << self.rev).s.recip

    @property
    def dual(self):
        return self << CgaExpr([(AXES, 1)])

    def grade_project(self, grade):
        return CgaExpr({k: v for k, v in self.terms.items() if len(k) == grade})


NI = CgaExpr([('-', 1), ('+', 1)])
NO = CgaExpr([('-', 1), ('+', -1)]) * 0.5
MINKOWSKI_PLANE = CgaExpr([('-+', 1)])
X = CgaExpr([('x', 1)])
Y = CgaExpr([('y', 1)])
Z = CgaExpr([('z', 1)])


def cga_from_latex(s) -> CgaExpr:
    pass


def point(subscript):
    if subscript:
        subscript = '_' + subscript
    return point_at(ScalarExpr(f'a{subscript}'), ScalarExpr(f'b{subscript}'), ScalarExpr(f'c{subscript}'))


def point_at(x, y=0, z=0):
    mag2 = x*x  # + y*y + z*z
    return NO + CgaExpr([('x', x)]) + NI * 0.5 * mag2


def sphere_at(p, subscript):
    r = ScalarExpr(f'r')
    return p - NI * 0.5 * r*r


s1 = sphere_at(point_at(-1), 1)
s2 = sphere_at(point_at(1), 2)
p = point_at(ScalarExpr('a'), ScalarExpr('b'))


# def unit_sphere():
#     return NO - NI * 0.5


# print(s1 * s2 * p * s2 * s1)

# pp = CgaExpr([
#     ('xy', -9),
#     ('x-', -9),
#     ('y+', -9),
#     ('y-', -9),
#     ('+-', 9),
# ])
# print(pp)
# print()
# r = 9
# m = (NI << pp).inv
# print((pp-r)*m)
# print()
# print()
# print((pp+r)*m)


# c1 = point('1')
# c2 = point('2')
c1 = NO
c2 = NI
alpha = CgaExpr([('', ScalarExpr(r'\alpha'))])

transform = alpha * c1 * c2 + c2 * c1

# print(transform)

# print(transform * point(''))

w1 = ScalarExpr('w_1')  # 1/w
w2 = ScalarExpr('w_2')  # 1/(1+wy)
x = ScalarExpr('q')
x_prime = x * w2
f = point_at(0, w1)

print(point('1') ^ NI)
# print(((point('1') ^ NI).dual ^ NO).dual)

# print(NI * NO)
# print(NO * NI)
