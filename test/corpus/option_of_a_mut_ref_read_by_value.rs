// What an `Option<&mut T>` or a `Result<&mut T, E>` holds, read by value:
// `copied()`, `cloned()`, `map(|x| *x)` and `as_deref()`, of an item a
// `&mut` reaches, a number's, a `String`'s, an `Option`'s, a generic `T`'s.
// Each `&mut` to a number is a handle, `{ value }`: each reads its `value`,
// never gives the handle on (#1311).
fn copied(o: Option<&mut i32>) -> Option<i32> {
    o.copied()
}
fn cloned(o: Option<&mut i32>) -> Option<i32> {
    o.cloned()
}
fn mapped(o: Option<&mut i32>) -> Option<i32> {
    o.map(|x| *x)
}
fn unwrapped(o: Option<&mut i32>) -> i32 {
    *o.unwrap()
}
fn deref(o: Option<&mut String>) -> Option<&str> {
    o.map(|s| s.as_str())
}
fn as_deref(o: &Option<Box<i32>>) -> Option<&i32> {
    o.as_deref()
}
fn result(r: Result<&mut i32, ()>) -> Result<i32, ()> {
    r.copied()
}
fn nested(o: Option<&mut Option<i32>>) -> Option<Option<i32>> {
    o.copied()
}
fn text(o: Option<&mut String>) -> Option<String> {
    o.cloned()
}
fn as_deref_mut_ref(o: Option<&mut i32>) -> Option<i32> {
    o.as_deref().copied()
}
fn generic<T: Copy>(o: Option<&mut T>) -> Option<T> {
    o.copied()
}
fn generic_mapped<T: Copy>(o: Option<&mut T>) -> Option<T> {
    o.map(|x| *x)
}
fn generic_result<T: Clone, E>(r: Result<&mut T, E>) -> Result<T, E> {
    r.cloned()
}

fn main() {
    let mut n = 5;
    let mut s = String::from("hi");
    let b = Some(Box::new(7));
    println!("{:?}", copied(Some(&mut n)));
    println!("{:?}", cloned(Some(&mut n)));
    println!("{:?}", mapped(Some(&mut n)));
    println!("{:?}", unwrapped(Some(&mut n)));
    println!("{:?}", deref(Some(&mut s)));
    println!("{:?}", as_deref(&b));
    println!("{:?}", result(Ok(&mut n)));
    println!("{:?}", result(Err(())));
    println!("{:?} {:?}", copied(None), mapped(None));
    let mut inner = Some(3);
    let mut empty: Option<i32> = None;
    println!("{:?} {:?} {:?}", nested(Some(&mut inner)), nested(Some(&mut empty)), nested(None));
    println!("{:?}", text(Some(&mut s)));
    println!("{:?}", as_deref_mut_ref(Some(&mut n)));
    let mut v = vec![1u8, 2];
    println!("{:?} {:?}", generic(Some(&mut n)), generic(v.first_mut()));
    println!("{:?} {:?}", generic_mapped(Some(&mut n)), generic_mapped(Some(&mut s.len())));
    println!("{:?} {:?}", generic_result::<_, ()>(Ok(&mut n)), generic_result::<String, _>(Err(4)));
    println!("{:?}", generic_result::<_, ()>(Ok(&mut s)));
    let mut items = vec![1, 2, 3, 4];
    let mut it = items.iter_mut();
    let a = it.next().copied();
    let b = it.nth(0).copied();
    let c = it.last().copied();
    let d = items.iter_mut().nth(1).map(|x| *x);
    let e = items.first_mut().copied();
    let f = items.iter_mut().nth(2).cloned();
    let g = items.last_mut().map(|x| *x + 1);
    let h = items.get_mut(1).copied().unwrap_or(0);
    println!("{a:?} {b:?} {c:?} {d:?} {e:?} {f:?} {g:?} {h}");
}
