// test float division override
assert_eq(1/2, 0.5);
assert_eq(1, 1.0);

let v = vec([1.0, 2.0]);
print(v.x);

print(v.cross(vec([3, 4])));
print(v);

print("hi");

assert_eq(vec(3, 5) * 2, vec(6, 10, 0));
assert_eq(2 * vec(3, 5), vec(6, 10));


print("hello" + (0.1 + 0.2 == 0.3));
print("hello" + (0.1 + 0.2 > 0.3));


let funny = 0.1 + 0.2;
print(funny);
while funny == 0.3 {
    funny = funny * 2 - 0.3;
}
print(funny);
print(funny == 0.3);
print(0.3);

assert_eq(vec(0.1, 0.2) + vec(0.2, 0.1), vec(0.3, 0.3));
assert_neq(vec(0.1, 0.2) + vec(0.2, 0.1), vec(0, 0, 0.2));
print(1.0/0.0);
print(0.0/0.0);

// assert(0.3 != 0.1 + 0.2);
// assert(0.3 ~ 0.1 + 0.2);
// assert(2 ~ 3);

assert_neq(point(1, 2), point(2, 4));
assert_eq(point(1, 2), point(1, 2));
assert_neq(point(1, 2), vec(1, 2));
print(point(1, 2));

assert_eq(point(-3) + vec(1, 2), point(-2, 2))
