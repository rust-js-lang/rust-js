// A `&mut` of an object given to generic code that keeps it past the call,
// `set_get`'s, which gives it back: an in-place cell of the object, whose
// write of the whole, `*a = v`, changes the caller's object, a parameter's
// as a local's, not the variable that names it here (#528, ADR 0454).
fn set_get<'a, T>(a: &'a mut T, v: T) -> &'a mut T {
    *a = v;
    a
}

fn nested<'a, T>(a: &'a mut T, v: T) -> &'a mut T {
    set_get(a, v)
}

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy)]
struct Pair(i32, i32);

fn through_param(v: &mut Vec<i32>) {
    set_get(v, vec![3, 4]);
}

fn through_param_used(v: &mut Vec<i32>) {
    let r = set_get(v, vec![5, 6]);
    r.push(7);
}

fn struct_through_param(p: &mut Point) {
    set_get(p, Point { x: 8, y: 9 }).y += 1;
}

fn main() {
    let mut v1 = vec![1, 2];
    through_param(&mut v1);
    println!("{v1:?}");

    let mut v2 = vec![1, 2];
    set_get(&mut v2, vec![3, 4]);
    println!("{v2:?}");

    let mut v3 = vec![1, 2];
    let r = set_get(&mut v3, vec![3, 4]);
    r.push(5);
    println!("{v3:?}");

    let mut v4 = vec![1, 2];
    through_param_used(&mut v4);
    println!("{v4:?}");

    let mut s = String::from("a");
    set_get(&mut s, String::from("b")).push('c');
    println!("{s}");

    let mut p = Point { x: 1, y: 2 };
    set_get(&mut p, Point { x: 3, y: 4 }).x += 10;
    println!("{p:?}");

    let mut q = Point { x: 1, y: 2 };
    struct_through_param(&mut q);
    println!("{q:?}");

    let mut pair = Pair(1, 2);
    let before = pair;
    set_get(&mut pair, Pair(3, 4)).0 += 1;
    println!("{before:?} {pair:?}");

    let mut v5 = vec![1];
    nested(&mut v5, vec![8, 9]).push(10);
    println!("{v5:?}");

    let mut n = 1;
    *set_get(&mut n, 2) += 5;
    println!("{n}");
}
