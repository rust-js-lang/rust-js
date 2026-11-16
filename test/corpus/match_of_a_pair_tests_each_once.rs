// A `match` of a pair of options, as React's Fizz server writes
// `trackPostpone`: MIR tests the first's two ways, `if (a == null) { .. }
// else if (a != null) { .. }`, and its second way is the `else`, so the
// chain reads `else if (keyPath != null)`, the first one's test not again
// (#1972). A test the program writes itself, `else if path.is_some()`, a
// condition of its own, stays as written.
use std::cell::RefCell;
use std::rc::Rc;

struct Tracked {
    content_key_path: RefCell<Option<Rc<u32>>>,
}

fn same(tracked: Option<&Tracked>, key_path: &Option<Rc<u32>>) -> bool {
    if let Some(tracked) = tracked
        && match (&*tracked.content_key_path.borrow(), key_path) {
            (Some(content_key_path), Some(key_path)) => Rc::ptr_eq(content_key_path, key_path),
            (None, None) => true,
            _ => false,
        }
    {
        return true;
    }
    false
}

fn written(path: Option<u32>, deep: bool, out: &mut Vec<&'static str>) {
    if path.is_none() {
        out.push("root");
    } else if path.is_some() {
        out.push(if deep { "deep" } else { "shallow" });
    } else {
        out.push("never");
    }
    out.push("end");
}

fn main() {
    let key = Rc::new(7);
    let tracked = Tracked {
        content_key_path: RefCell::new(Some(Rc::clone(&key))),
    };
    let empty = Tracked {
        content_key_path: RefCell::new(None),
    };
    println!(
        "{} {} {} {}",
        same(Some(&tracked), &Some(Rc::clone(&key))),
        same(Some(&tracked), &Some(Rc::new(7))),
        same(Some(&empty), &None),
        same(None, &None)
    );
    let mut out = Vec::new();
    written(None, true, &mut out);
    written(Some(1), true, &mut out);
    written(Some(1), false, &mut out);
    println!("{out:?}");
}
