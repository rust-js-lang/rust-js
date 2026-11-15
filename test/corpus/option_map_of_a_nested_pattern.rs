// An `Option`'s `map` whose closure takes its value apart past one level:
// a one-element tuple in a pair, `|((k,), x)|`, and in a one-element tuple
// in another, `|(((k,),),)|`, a struct in a tuple, a tuple in a struct, and
// of a type parameter's. Its lowering took a part apart only at the top, so
// `k` was a name nothing declared, and the JS was refused (#1900, itertools
// 0.10.5's `cons_tuples`). Each part is what it reads now, however deep, as
// a closure inlined where it's called reads it.
#[derive(Debug, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Labelled {
    label: &'static str,
    at: (i32, (u8,)),
}

fn pair(o: Option<((u8,), char)>) -> Option<(u8, char)> {
    o.map(|((k,), x)| (k, x))
}

fn deep(o: Option<(((u16,),),)>) -> Option<u16> {
    o.map(|(((k,),),)| k + 1)
}

fn struct_in_tuple(o: Option<(Point, bool)>) -> Option<i32> {
    o.map(|(Point { x, y }, flip)| if flip { y - x } else { x - y })
}

fn tuple_in_struct(o: Option<Labelled>) -> Option<String> {
    o.map(|Labelled { label, at: (n, (m,)) }| format!("{label} {n} {m}"))
}

fn generic<K, X: std::fmt::Debug>(o: Option<((K,), X)>) -> Option<(K, String)> {
    o.map(|((k,), x)| (k, format!("{x:?}")))
}

fn main() {
    let k = 99;
    println!("{k} {:?} {:?}", pair(Some(((1,), 'a'))), pair(None));
    println!("{:?} {:?}", deep(Some((((41,),),))), deep(None));
    let p = Point { x: 3, y: 10 };
    println!("{:?} {:?} {:?}", struct_in_tuple(Some((p, true))), struct_in_tuple(Some((p, false))), struct_in_tuple(None));
    let l = Labelled { label: "here", at: (5, (6,)) };
    println!("{:?} {:?}", tuple_in_struct(Some(l)), tuple_in_struct(None));
    println!("{:?} {:?}", generic(Some(((String::from("s"),), [1, 2]))), generic::<u8, ()>(None));
}
