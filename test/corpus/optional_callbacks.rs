// An `Option` of a type parameter bound by `FnOnce`, `FnMut` or `Fn` is
// the function or `None`: a function is never `undefined` or `null`, so
// its `Some` isn't boxed and what it holds is read as it is. React's
// `flushSyncFromReconciler(fn)`, which calls `fn` where it's given one.
// What it gives, an `R`, may be an `Option` itself, whose `Some` is boxed.
fn flush_sync<R>(f: Option<impl FnOnce() -> R>) -> Option<R> {
    if let Some(f) = f { Some(f()) } else { None }
}

fn each<F: FnMut(i32)>(items: &[i32], mut f: Option<F>) -> usize {
    let mut seen = 0;
    for &item in items {
        if let Some(f) = f.as_mut() {
            f(item);
            seen += 1;
        }
    }
    seen
}

fn twice<F>(f: Option<&F>, n: i32) -> Option<i32>
where
    F: Fn(i32) -> i32,
{
    f.map(|f| f(f(n)))
}

fn main() {
    let three = flush_sync(Some(|| 3));
    let none = flush_sync(None::<fn() -> i32>);
    let some_none = flush_sync(Some(|| None::<i32>));
    let some_five = flush_sync(Some(|| Some(5)));
    let unit = flush_sync(Some(|| ()));
    println!("{three:?} {none:?} {some_none:?} {some_five:?} {unit:?}");

    let mut total = 0;
    let seen = each(&[1, 2, 3], Some(|item| total += item));
    println!("{seen} {total}");
    println!("{}", each(&[1, 2], None::<fn(i32)>));

    let add = |n| n + 1;
    let added = twice(Some(&add), 1);
    let not_added = twice(None::<&fn(i32) -> i32>, 1);
    println!("{added:?} {not_added:?}");
}
