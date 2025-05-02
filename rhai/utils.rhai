import "utils/layers" as layers;

fn lerp(a, b, t) { a + (b-a)*t }

fn nth_uppercase_name(n) {
    assert_eq(type_of(n), "i64");

    let ret = "";
    while n > 0 {
        n -= 1;
        ret = to_char('A'.to_int() + (n % 26)) + ret;
        n = floor(n / 26);
    }
    ret
}

fn uppercase_name_to_n(name) {
    assert_eq(type_of(n), "string");

    let ret = 0;
    for c in name {
        ret = ret * 26 + c.to_int() - 'A'.to_int() + 1;
    }
    ret
}
