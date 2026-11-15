// An `Option` local lent as a `&mut` that never takes its `Some` away, a
// `match &mut found`'s, React's getParentFragmentInstances', or
// `get_or_insert_with`'s, React's Flight client's `deferred`: its own writes
// say what it is, returned from a nullable function as it is; one whose
// lend writes `None`, `take()` or `*lent = None`, given `null` (#1933).
#[cfg_attr(rust_js, rust_js::nullable)]
fn found(items: &[u32]) -> Option<Vec<u32>> {
    let mut found = None;
    for &item in items {
        match &mut found {
            None => found = Some(vec![item]),
            Some(found) => found.push(item),
        }
    }
    found
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn deferred(items: &[u32], clear: bool) -> Option<Vec<u32>> {
    let mut deferred = found(&[]);
    for &item in items {
        deferred.get_or_insert_with(Vec::new).push(item);
    }
    if clear {
        deferred.take();
    }
    deferred
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn emptied(items: &[u32]) -> Option<usize> {
    let mut first = found(items).map(|items| items.len());
    if first.is_some() {
        let lent = &mut first;
        *lent = None;
        return first;
    }
    None
}

fn main() {
    println!("{:?} {:?}", found(&[1, 2]), found(&[]));
    println!("{:?} {:?} {:?}", deferred(&[3], false), deferred(&[3], true), deferred(&[], false));
    println!("{:?} {:?}", emptied(&[4]), emptied(&[]));
}
