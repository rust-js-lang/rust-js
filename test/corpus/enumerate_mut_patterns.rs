// A pattern of two bindings of one element, `(i, (a, b))` of
// `iter_mut().enumerate()`, which MIR reads twice: one name in the loop's
// head, which the body reads for both.
#[derive(Debug)]
struct P {
    x: i32,
    y: i32,
}

fn main() {
    let mut pairs = vec![(1, 2), (1, 3)];
    for (i, (n, _c)) in pairs.iter_mut().enumerate() {
        *n += i as i32;
    }
    println!("{pairs:?}");

    let mut ps = vec![P { x: 1, y: 2 }, P { x: 3, y: 4 }];
    for (i, P { x, y }) in ps.iter_mut().enumerate() {
        *x += i as i32;
        *y *= 2;
    }
    println!("{ps:?}");

    let item = [(1, 'a'); 3];
    let mut v = item.to_vec();
    for (i, (a, b)) in v.iter_mut().enumerate().skip(1) {
        *a *= i as i32 + 1;
        *b = 'z';
    }
    println!("{v:?}");
}
