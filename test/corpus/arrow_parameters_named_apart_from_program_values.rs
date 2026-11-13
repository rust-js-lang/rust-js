// An arrow the compiler makes around a value the program gives, a
// `next_if_eq`'s test, an `inspect`'s, a sort's by a key, a `contains`, a
// `binary_search`'s, a `for_each`'s, a char set's pattern, a `retain`'s,
// names its parameter apart from what that value reads: a variable the
// program calls `item`, `a`, `each`, `x`, `c` or `kept` is never hidden by
// it, `(item$1) => item$1 === item`, where `(item) => item === item` was
// always true (#1667).
#[derive(Debug, PartialEq, Clone)]
struct P(i32);

fn main() {
    let item = 5;
    let mut it = [1, 5, 3].into_iter().peekable();
    println!("{:?} {:?}", it.next_if_eq(&item), it.next_if_eq(&item));
    let item = |x: &i32| println!("saw {x}");
    let v: Vec<i32> = [3, 1, 2].into_iter().inspect(item).collect();
    println!("{v:?}");
    let a = |x: &i32| -*x;
    let mut w = vec![3, 1, 2];
    w.sort_by_key(a);
    println!("{w:?}");
    let a = |x: &P| -x.0;
    let mut ps = vec![P(3), P(1), P(2)];
    ps.sort_by_key(a);
    println!("{ps:?}");
    let each = P(2);
    println!("{}", [P(1), P(2)].contains(&each));
    let each = 2;
    println!("{:?}", [1, 2, 3].binary_search(&each));
    let each = P(3);
    println!("{:?}", [P(1), P(3)].iter().position(|p| *p == each));
    let item = |p: &P| p.0;
    println!("{:?}", [P(1), P(3)].binary_search_by_key(&3, item));
    let x = |n: i32| println!("each {n}");
    [7, 8].into_iter().for_each(x);
    let c = ['a', 'b'];
    println!("{}", "abxba".trim_matches(&c[..]));
    let mut kept = String::from("a1b2");
    kept.retain(|ch| ch.is_alphabetic());
    println!("{kept}");
    let nested: Vec<Vec<i32>> = vec![vec![1], vec![2, 3]];
    let item = |v: Vec<i32>| v;
    let flat: Vec<i32> = nested.into_iter().flat_map(item).collect();
    println!("{flat:?}");
}
