# hyperpuzzlescript

Domain-specific language for defining puzzles for Hyperspeedcube

## Learn Hyperpuzzlescript in Y minutes

Based on [Learn X in Y Minutes](https://learnxinyminutes.com/)

### Basics

```c
// single-line comment
/*
multi-line comment
*/

// whitespace is insignificant, except newlines outside parentheses

// variables are dynamically typed
some_string = "strings use double quotes"
another_string = "${some_string} and support interpolation" ++ " and concatenation" ++ "
and literal newlines
and \"escapes\" like \\ (backslash), \n (newline), and \t (tab)
"

// all numbers are 64-bit floating-point
// there are three number types: Nat ⊂ Int ⊂ Num
some_var = 12.1
// type annotations are optional and are checked at runtime
my_number: Num = 3.5
my_integer: Int = 4
// variables can be reassigned, even to different types
my_number: Nat = 3.0 // equivalent to 3

if my_number == 3 {
    print("yes!")
} else if my_number != 4 {
    print("maybe!")
} else {
    print("no!")
}

// if-expressions are allowed, as long as each branch contains only a single expression
print(if my_number == 3 { "yes" } else { "no!" })

// it is an error to use an uninitialized variable
// but variables may be assigned `null`, a special value of type `Null`
this_var: Null = null
// type unions use `|`
// if-expressions without an `else` branch return `null`
might_be_null: Int | Null = if my_number == 4 { "weird" }

// variables cannot be used outside the scope where they are defined
// but variables from outer scopes may be overwritten
outer_var = 14
{
    outer_var = 10
    inner_var = 2
}
assert_eq(outer_var, 10)
// inner_var is undefined here

// print() outputs to console
print(another_string, 14) // separated by spaces
```

### Math

```c

// numeric comparisons use approximate equality
assert(0.1 + 0.2 == 0.3)
assert(str(0.1 + 0.2) != str(0.3))
// see https://0.30000000000000004.com/ if this is surprising to you

// several math constants are built in with names and symbols
assert_eq(pi, π)
assert_eq(tau, τ)
assert_eq(π*2, τ)
assert_eq(phi, φ)
assert_eq(inf, ∞)
assert(2.71 < exp(1) and exp(1) < 2.72) // Euler's constant
// `+` is a no-op
assert_eq(17, +17)
assert_neq(+∞, -∞)
// `sqrt()` and `√` are equivalent
assert_eq(sqrt(2), √2)
assert_eq(φ, (1+√5)/2)
// `^` for exponentiation
assert_eq(2^4, 16)
// multiply by `deg` to convert degrees to radians
assert_eq(30*deg, pi/6)
// or use `°`
assert_eq(30°, π/6)
// division never rounds
assert_eq(5/2, 2.5)
// `floor()`, `ceil()`, `trunc()`, and `round()` for rounding
assert_eq(floor(5/3), 1) // rounds toward -∞
assert_eq(ceil(5/3), 2) // rounds toward ∞
assert_eq(round(5/3), 2) // rounds toward nearest (prefers away from 0)
assert_eq(trunc(5/3), 1) // rounds toward 0
// `%` performs Euclidean remainder
a = -10
b = 3
assert_eq(floor(a / b) * b + a % b, a) // true for all inputs
assert_eq(1/0, ∞) // division by 0 results in infinity
// dividing 0 by 0 emits an error

// other math functions:
assert_eq(abs(a), 10)
assert_eq(sign(a), -1)
assert_eq(cbrt(10), 10^(1/3))
assert_eq(factorial(4), 24)
assert(is_even(a))
assert(is_odd(b))
assert_eq(min(a, b), -10)
assert_eq(max(a, b), 3)
assert_eq(clamp(55, a, b), b) // requires a <= b
assert(is_infinite(-∞))
assert(is_finite(999))
assert_eq(ln(99)/ln(10), log10(99))
assert_eq(ln(99)/ln(2), log2(99))
assert_eq(exp2(7), 2^7)
assert_eq(exp(1)^4, exp(4))

// all trigonometric functions are available, including inverses, reciprocals, and inverse reciprocals
assert_eq(sin(π/2), 1)
assert_eq(cos(π/2), 0)
assert_eq(tan(30°), 1/√3)
assert_eq(cot(1.62), 1/tan(1.62))
assert_eq(acos(1/3), arccos(1/3)) // inverse trig may use long or short names
assert_eq(sinh(5), (exp(5) - exp(-5))/2) // hyperbolic trig
```

### Collections

```c
// there are two list types: NonEmptyList ⊂ List
some_list: List = [1, 2, "hello", "world"]
assert_eq(1, some_list[0]) // lists are zero-indexed
assert_eq(some_list[-1], "world") // negative numbers count from the end
assert_eq(null, some_list[4]) // out-of-bounds indices are null
// element type may be specified
some_typed_list: List[Str | Nat] = ["a", 4] // -2 here would cause an error

// `??` is the "null-coalescing" operator
assert_eq("world", some_list[3] ?? 42)
assert_eq(42, some_list[4] ?? 42)

// all functions can be used with `.` notation
assert_eq(some_list.len(), len(some_list))
assert_eq((π/2).sin().acos(), acos(sin(π/2)))

// boolean operators use words
assert(true or false)
assert(not (true and false))

// maps use string keys. values may be any type.
some_map: Map = #{
  hello = 1,
  "all keys are strings" = "that's right!",
  "${some_list[3]}" = some_list,
}
print(some_map) // prints contents of the map
// values can be accessed in several ways:
assert_eq(some_map.hello, some_map["hello"])
use world, hello as renamed_var from some_map // introduces/assigns variables
assert_eq(renamed_var, 1)
assert_eq(world, some_list)
// missing keys are null
assert_eq(some_map.not_present, null)

// use concatenation to append to a list
some_list = some_list ++ ["new", "values"]
some_list ++= ["are", "here"]
// copies of the list stay unmodified
assert_eq(some_list.len(), 8)
assert_eq(world.len(), 4)

// iterate over lists
for elem in world {
    print(elem)
}
// iterate over lists with index
for i, elem in world {
    assert_eq(world[i], elem)
}

assert_eq(sorted(["b", "a", "c"]), ["a", "b", "c"])
assert_eq(rev([1, 2, 3]), [3, 2, 1])
assert_eq(sorted(keys(some_map)), ["all keys are strings", "hello", "world"])

// ranges can include or exclude the last number
assert_eq(0..5, [0, 1, 2, 3, 4])
assert_eq(0..=5, [0, 1, 2, 3, 4, 5])
// backward ranges are empty
assert_eq(5..0, [])
// use `rev()` to iterate backward
for i in rev(0..5) {
    print(i) // 4, 3, 2, 1, 0
}

// iterate over maps
for key, value in some_map {
    print(key, value)
}

// `*` to "splat" a list
pair = [10, 20]
assert_eq([1, 2, *pair, 3, 4], [1, 2, 10, 20, 3, 4])
assert_eq(min(*pair), 10)
// `**` to "splat" a map
m = #{a = 4, b = 8, c = 12}
assert_eq(#{**m, q = 14}, #{a = 4, b = 8, c = 12, q = 14})

// lists and maps can be destructured, even with splats
[one, two, *remaining] = 1..=5
assert_eq(one, 1)
assert_eq(two, 2)
assert_eq(remaining, [3, 4, 5])
// when destructuring without a splat, the length must match
[x, y, z] = [1, 2, 3]
// destructuring maps sets nonexistent keys to null
#{b, c, nonexistent, **rest_of_the_map} = m
assert_eq([b, c, nonexistent], [8, 12, null])
```

### Functions

```c
// functions use Rust-like notation, but type annotations are optional
fn summon(name: Str) -> NonEmptyList[Str] {
    // repr() puts a string in quotes and escapes any characters
    print("saying ${repr(name)} three times ...")
    return [name, name, name]
}
// functions can be overloaded, as long as there's no overlap in the ways the function can be called
fn summon() {
    // if the function body is a single expression, `return` may be omitted
    summon("Beetlejuice") // returns a list of 3 strings
}

// use `-> Null` to avoid implicitly returning a value
fn summon_beetlejuice_and_return_nothing() -> Null {
    summon()
}

// functions must be defined before they are called
summon()
summon("HactarCE")
assert_eq(null, summon_beetlejuice_and_return_nothing())

// functions are first-class values of type `Fn`
print(summon) // function with 2 overloads

// if `f` has no other overloads, then these two lines are equivalent:
fn add(a, b) { a + b }
add = fn(a, b) { a + b }

// functions can take other functions as arguments
fn do_10_times(f: Fn) {
    for i in 1..=10 { f(i) }
}
do_10_times(print) // 1, 2, 3, ...
do_10_times(fn(x) { print(1/x) }) // 1, 0.5, 0.3333...

fn reduce(op: Fn, args) {
    [init, *rest] = args
    ret = init
    for arg in rest {
        ret = op(ret, arg)
    }
    return ret
}
fn sum(list) { reduce(fn(a, b) { a + b }, list) }
fn product(list) { reduce(fn(a, b) { a * b }, list) }
assert_eq(sum(1..=10), 55)
assert_eq(product(1..=10), 3628800)

// to override a built-in function, first set it to `null`
factorial = null
fn factorial(n: Nat) { product(1..=n) }
assert_eq(factorial(10), 3628800)

// splat can be used to make variadic functions
fn reduce_variadic(op: Fn, first_arg, *args) {
    ret = first_arg
    for arg in args {
        ret = op(ret, arg)
    }
    return ret
}
assert_eq(reduce_variadic(max, 5, 2, 30, 6), 30)

// functions support keyword arguments
// keyword arguments can be made optional by supplying a default value
fn print_args(arg1, *other_args, kwarg1, kwarg2=10, **kwargs) {
    print(arg1) // required
    print(other_args) // list of remaining non-keyword arguments
    print(kwarg1) // required
    print(kwarg2) // optional; defaults to 10
    print(kwargs) // map of remaining keyword arguments
}

print_args(
    1, // arg1
    2, 3, // other_args
    kwarg1=4,
    x=5, y=6, z=7, // kwargs
)

// when not using `*args`, use `*` to separate non-keyword args from keyword args
fn keyword_only(*, k1, k2, **kwargs) {}
keyword_only(k1=1, k2=3)

// `export` is syntactic sugar for returning a map from a function
// `export` and `return` cannot be used in the same function
fn make_map_using_export(a) -> Map {
    export a // existing variable
    export b = 15 // new variable
    export fn f(x) { x + a } // functions can capture variables
    export c = f(b)
    assert_eq(c, 15 + a)
    // `export` supports all the same functionality as `use`
    export d as q, e from #{ d = 35, e = 40, x = 45 }
    // q and e are exported, but not acessible from the current scope
}
fn make_map_using_return(a) {
    fn f(x) { x + a }
    return #{
        a = a,
        b = 15,
        f = f,
        c = 15 + a,
        q = 35,
        e = 40,
    }
}
assert_eq(
    sorted(keys(make_map_using_export(5))),
    sorted(keys(make_map_using_return(5))),
)
```

### Special variables

```c
// special variables begin with `#`
// these variables can be accessed from within functions, but can only be overwritten by starting a new scope using `with`
fn regular_convex_polytopes() -> Nat {
    regular_convex_polytopes(#ndim)
}
fn regular_convex_polytopes(dimension_count: Nat) -> Nat {
    // fetch from the list; if that fails, use `3`
    [1, 1, ∞, 5, 6][dimension_count] ?? 3
}
fn regular_convex_polytopes(dimension_count: Null) {
    error("missing ndim")
}

with #ndim = 3 {
    assert_eq(regular_convex_polytopes(), 5)
}
with #ndim = 4 {
    assert_eq(regular_convex_polytopes(), 6)
}

// calling `regular_convex_polytopes()` here would emit an error saying "missing ndim"
```

### Modules

#### some_module.hps

```c
a = 10 // private
export b = 20 // public
assert_eq(b, 20) // `b` is also now a local variable

fn some_func() {} // private
export fn some_other_func(x) { x + 1 } // public
assert_eq(some_other_func(2), 3) // `some_other_func` is also now a local variable

export * from #{g=[1, 2], h=[3, 4]} // export `g` and `h`
// `g` and `h` are exported, but are not local variables
```

#### some_other_module.hps

```c
m = @learn_x_in_y/some_module
print(m) // just an ordinary map

// `some_module.hps` is only evaluated once
assert_eq(@learn_x_in_y/some_module.b, 20)
assert_eq(@learn_x_in_y/some_module.a, null) // private
assert_eq(@learn_x_in_y/some_module.nonexistent, null)

// use some or all entries from the module
use * from @learn_x_in_y/some_module
assert_eq(b, 20)
assert_eq(some_other_func(5), 6)
assert_eq(g, [1, 2])
assert_eq(h, [3, 4])
```

### Euclidean geometry

```c
// import functions and operator overloads
use * from euclid

v = vec(1, 3)
u = vec(1)
assert_eq(vec(1), vec(1, 0))
assert_eq(u + v, vec(2, 3))
assert_eq(u - v, vec(0, -3))
assert_eq(v * 4, vec(4, 12))
assert_eq(-u, vec(-1))

// TODO: more math: point(), plane(), distance(), rot(), ...
```
