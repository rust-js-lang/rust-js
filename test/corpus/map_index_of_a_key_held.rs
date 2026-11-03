// A map's `m[k]` where a test shows it holds `k` (rd558): `contains_key(k)`
// holding, with nothing between that may take `k` out or change `k`, so
// the index can't panic, is the map's `get(k)`, a stored `None` too, as
// React's ReactMarkupServer reads `stashedErrors.get(id)` after
// `stashedErrors.has(id)`; a `RefCell`'s map by the cell (ADR 0881). A
// different key, a different map, a `remove`, a key set again, a call that
// may change the cell, or a drop that does, between: the index is checked,
// and panics as Rust's does.
use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

type Errors = Rc<RefCell<HashMap<String, Option<u32>>>>;

/// Empties its map where it's dropped.
struct Clearer(Errors);

impl Drop for Clearer {
    fn drop(&mut self) {
        self.0.borrow_mut().clear();
    }
}

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn held(counts: &HashMap<String, u32>, key: &str) -> u32 {
    if counts.contains_key(key) { counts[key] } else { 0 }
}

fn held_none(values: &HashMap<u32, Option<u32>>, key: u32) -> Option<u32> {
    if values.contains_key(&key) { values[&key] } else { Some(9) }
}

fn held_past_a_return(counts: &BTreeMap<u32, u32>, key: u32) -> u32 {
    if !counts.contains_key(&key) {
        return 0;
    }
    counts[&key] + 1
}

fn held_in_a_loop(counts: &HashMap<String, u32>, keys: &[&str]) -> u32 {
    let mut total = 0;
    for &key in keys {
        if !counts.contains_key(key) {
            continue;
        }
        total += counts[key];
    }
    total
}

fn held_by_a_guard(errors: &Errors, id: &str) -> Option<u32> {
    let all = errors.borrow();
    if all.contains_key(id) { all[id] } else { Some(9) }
}

fn held_on_one_path(counts: &HashMap<String, u32>, key: &str, tested: bool) -> u32 {
    if tested && !counts.contains_key(key) {
        return 0;
    }
    counts[key]
}

fn another_key(counts: &HashMap<String, u32>, key: &str, other: &str) -> u32 {
    if counts.contains_key(key) { counts[other] } else { 0 }
}

fn another_map(counts: &HashMap<String, u32>, others: &HashMap<String, u32>, key: &str) -> u32 {
    if counts.contains_key(key) { others[key] } else { 0 }
}

fn removed_between(counts: &mut HashMap<String, u32>, key: &str) -> u32 {
    if counts.contains_key(key) {
        counts.remove(key);
        return counts[key];
    }
    0
}

fn removed_on_the_right(counts: &mut HashMap<String, u32>, key: &str) -> u32 {
    if counts.contains_key(key)
        && {
            counts.clear();
            true
        }
    {
        counts[key]
    } else {
        0
    }
}

fn key_set_between(counts: &HashMap<String, u32>, mut key: &str, other: &'static str) -> u32 {
    if counts.contains_key(key) {
        key = other;
        return counts[key];
    }
    0
}

fn called_between(errors: &Errors, id: &str, clear: &dyn Fn()) -> Option<u32> {
    let mut error = Some(9);
    if errors.borrow().contains_key(id) {
        clear();
        error = errors.borrow()[id];
    }
    error
}

fn dropped_between(errors: &Errors, id: &str, clearer: Clearer) -> Option<u32> {
    if errors.borrow().contains_key(id) {
        {
            let _gone = clearer;
        }
        return errors.borrow()[id];
    }
    Some(9)
}

fn main() {
    let counts: HashMap<String, u32> = [("a".to_string(), 1), ("b".to_string(), 2)].into();
    println!("held: {}", held(&counts, "b"));
    println!("held, missing: {}", held(&counts, "z"));
    let values: HashMap<u32, Option<u32>> = [(1, None), (2, Some(5))].into();
    println!("held none: {:?}", held_none(&values, 1));
    println!("held some: {:?}", held_none(&values, 2));
    println!("held none, missing: {:?}", held_none(&values, 3));
    let sorted: BTreeMap<u32, u32> = [(1, 10), (2, 20)].into();
    println!("held past a return: {}", held_past_a_return(&sorted, 2));
    println!("held past a return, missing: {}", held_past_a_return(&sorted, 3));
    println!("held in a loop: {}", held_in_a_loop(&counts, &["a", "z", "b"]));

    let errors: Errors = Rc::new(RefCell::new(HashMap::new()));
    let handle_flight_error = {
        let errors = Rc::clone(&errors);
        move |id: &str, error: Option<u32>| {
            errors.borrow_mut().insert(id.to_string(), error);
        }
    };
    handle_flight_error("1", None);
    handle_flight_error("2", Some(7));
    // As ReactMarkupServer's `handleError` reads what Flight stashed.
    let handle_error = {
        let errors = Rc::clone(&errors);
        move |digest: Option<&str>, error: Option<u32>| -> Option<u32> {
            let mut error = error;
            if let Some(id) = digest
                && errors.borrow().contains_key(id)
            {
                error = errors.borrow()[id];
            }
            error
        }
    };
    println!("stashed none: {:?}", handle_error(Some("1"), Some(3)));
    println!("stashed some: {:?}", handle_error(Some("2"), Some(3)));
    println!("not stashed: {:?}", handle_error(Some("3"), Some(3)));
    println!("no digest: {:?}", handle_error(None, Some(3)));
    println!("held by a guard: {:?}", held_by_a_guard(&errors, "2"));

    check("held on one path", || held_on_one_path(&counts, "a", true));
    check("held on no path", || held_on_one_path(&counts, "z", false));
    check("another key", || another_key(&counts, "a", "z"));
    check("another map", || another_map(&counts, &HashMap::new(), "a"));
    check("removed between", || removed_between(&mut counts.clone(), "a"));
    check("removed on the right", || removed_on_the_right(&mut counts.clone(), "a"));
    check("key set between", || key_set_between(&counts, "a", "z"));
    let cleared = Rc::clone(&errors);
    check("called between", || {
        called_between(&errors, "2", &|| cleared.borrow_mut().clear())
    });
    handle_flight_error("4", Some(4));
    check("dropped between", || {
        dropped_between(&errors, "4", Clearer(Rc::clone(&errors)))
    });
}
