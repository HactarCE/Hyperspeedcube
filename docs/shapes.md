# Hyperspeedcube shape generator

## Preface

The goal of this document is to explain the entirety of the Hyperspeedcube puzzle engine. The reader of assumed to be familiar with basic linear algebra and have some geometric intuition. In particular, they should be comfortable with vectors as a representation of directions and points in 2D and 3D space, matrices as a representation of linear transformations, and the determinant as a representation of signed area/volume.

## Vectorspace Geometric Algebra

Complex numbers are a system where each number has two components: real and imaginary. Geometric Algebra is like complex numbers, but adds _way_ more components.[^complex] For example, 3D VGA (Vectorspace Geometric Algebra) adds _seven_ extra components a total of eight.

[^complex]: In fact, complex numbers _are_ a geometric algebra! You can think of them either as a 1D GA with a single extra basis vector $i^2=-1$, or as the even subalgebra of 2D VGA -- but I'm getting ahead of myself.

All geometric algebra systems have **scalars**, the ordinary numbers you know and love. In 3D VGA, we also have **vector** components $x$, $y$, and $z$, and we can build vectors out of them. For example, $x-7z$ represents the vector $\langle 1, 0, -7 \rangle$. Just like a complex number has both real and imaginary components (e.g., $3+2i$), a **multivector** represented in VGA can have both scalar and vector components (e.g., $5+3x+y-2z$). Each of these components is a **term**

You might recognize this as a [vector space], which just means that addition, subtraction, and scalar multiplication all work how you expect. But 3D VGA isn't actually a 3D vector space, because we have $1$ (the unit scalar) as a separate basis vector, orthogonal to $x$, $y$, and $z$. But don't worry, you can still visualize it using just three dimensions by treating the scale component as separate.

[vector space]: https://en.wikipedia.org/wiki/Vector_space

Sidenote: the term "vector" is a bit confusing. A "multivector" is any element of the vector space, while a "vector" in GA consists of only vector components, like $2x-y$ (but not $4+2x-y$). But the term "basis vector" applies in the vector space as a whole, so $1$, $x$, $y$, and $z$ are all **basis vectors**. Blame the mathematicians, not me.

So we have four basis vectors: $1$, $x$, $y$, and $z$. But 3D VGA actually has _eight_ basis vectors. How do we get all those extra basis vectors?

### Geometric product

(From now on, assume that letters $a$, $b$, $c$, and $d$ represent arbitrary vectors, and the letters $A$, $B$, $C$, and $D$ represent arbitrary multivectors.)

The geometric product is how we generalize multiplication to work on multivectors. We write it using ordinary multiplication, so $AB$ is the geometric product of $A$ and $B$. For scalars, it does what you expect. The geometric product has some nice properties:

- :white_check_mark: Associativity[^assoc]: $A(BC) = (AB)C$
- :white_check_mark: Distributivity: $A(B + C) = AB + AC$ and $(B + C)A = BA + CA$
- :x: No commutativity: $AB = BA$ isn't always true

[^assoc]: If you ever see a non-associative algebra (like the [octonions](https://en.wikipedia.org/wiki/Octonion)), run far, far away and never look back. They are utter hell.

The geometric product of anything involving a scalar is just scalar multiplication, which does commute: $x3 = 3x$. But what if we multiply two _vector_ components? If the vectors are the same, like $xx$, then they cancel and the result is $1$. If the vectors are different, like $xy$, then we get something new: a **bivector**. There are three unique bivectors in VGA: $xy$, $xz$, and $yz$. The way VGA works, $xy = -yx$, so we don't need a separate $yx$ component.

Let's make a multiplication table! Check for yourself that each entry in here makes sense. (left times top)

| $1$ |  $x$  |  $y$  | $z$  |
| --- | :---: | :---: | :--: |
| $x$ |  $1$  | $xy$  | $xz$ |
| $y$ | $-xy$ |  $1$  | $yz$ |
| $z$ | $-xz$ | $-yz$ | $1$  |

What happens when you multiply a bivector by a vector? Well when we multiply $xy$ by $x$, we get $xyx$. We can simplify $yx$ to $-xy$, and then $xx$ simplifies to $1$, so $xyx = x(-xy) = -xxy = -y$. But if we have three different letters, then it doesn't simplify: multiplying $xy$ by $z$ results in the **trivector** $xyz$, our eighth and final basis vector. In general, a geometric algebra with $n$ vector components will have $2^n$ basis vectors.

Let's make an even bigger multiplication table!

| $1$   |  $x$  |  $y$   |  $z$  | $xy$  |  $xz$  | $yz$  | $xyz$ |
| ----- | :---: | :----: | :---: | :---: | :----: | :---: | :---: |
| $x$   |  $1$  |  $xy$  | $xz$  |  $y$  |  $z$   | $xyz$ | $yz$  |
| $y$   | $-xy$ |  $1$   | $yz$  | $-x$  | $-xyz$ |  $z$  | $-xz$ |
| $z$   | $-xz$ | $-yz$  |  $1$  | $xyz$ |  $-x$  | $-y$  | $xy$  |
| $xy$  | $-y$  |  $x$   | $xyz$ | $-1$  | $-yz$  | $xz$  | $-z$  |
| $xz$  | $-z$  | $-xyz$ |  $x$  | $yz$  |  $-1$  | $-xy$ |  $y$  |
| $yz$  | $xyz$ |  $-z$  |  $y$  | $-xz$ |  $xy$  | $-1$  | $-x$  |
| $xyz$ | $yz$  | $-xz$  | $xy$  | $-z$  |  $y$   | $-x$  | $-1$  |

But what do all these components actually represent? Well just like the vector $x$ represents one unit of _distance_ in the positive direction along the X axis, the bivector $xy$ represents one unit of _positively-oriented area_ in the XY plane. And a trivector $xyz$ represents one unit of _positively-oriented volume_ in 3D space.

It's time for some new terminology:

- The **grade** of a multivector is the number of letters each component has. A scalar has grade 0, a vector has grade 1, a bivector has grade 2, a trivector has grade 3, etc.
- A **blade** is a multivector whose components all have the same grade.

To get a blade from a multivector, you can **grade-project** it, which extracts all the components of a particular grade. The projection of $A$ into grade $r$ is written $\langle A \rangle_r$

Most of the time, all the multivectors we see will be blades. The only exception to this rule is rotors, which we'll get to later.

### Outer product (wedge)

The outer product of $A$ and $B$ is written $A \wedge B$ ("$A$ wedge $B$").

When computing the geometric product of two multivectors, you get a lot of different components.

Given a blade $A$ with grade $s$ and a blade $B$ with grade $r$ is $\langle AB \rangle_{r+s}$. Here's some examples of how that works:

- The outer product of two vectors is always a bivector (or zero), with no scalar component.
- The outer product of a bivector and a vector is always a trivector (or zero).
- The outer product with a scalar is the same as ordinary multiplication by that scalar.

TODO: write about geometric intuition for this, and connection to the determinant.

### Inner product (dot)

TODO: write more

### Rotors

Now that you understand multivectors and the geometric, outer, and inner products, read [Marc ten Bosch's explanation of rotors][marctenbosch-rotors].

[marctenbosch-rotors]: https://marctenbosch.com/quaternions/#h_12
