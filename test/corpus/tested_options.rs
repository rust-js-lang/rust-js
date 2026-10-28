// An `Option` made where it's tested is its test (ADR 0910): `if let
// Some(x) = test.then_some(v)` is `if (test)`, `x` read as `v`, and so is
// a `let .. else` of one; where `v` may be `None` itself, or changes, or
// `x` is read past the test, the `Option` stays.

#[derive(Debug)]
struct Item {
    name: String,
}

fn named(item: &Item, wanted: bool) -> String {
    if let Some(found) = wanted.then_some(item) {
        format!("found {}", found.name)
    } else {
        "not wanted".to_string()
    }
}

fn first_name(items: &[Item], wanted: bool) -> &str {
    let Some(first) = wanted.then_some(&items[0]) else {
        return "none";
    };
    &first.name
}

// `v` may be `None`: the `Option`'s test is its own.
fn maybe(value: Option<&Item>, wanted: bool) -> String {
    let chosen = if wanted { value } else { None };
    match chosen {
        Some(item) => item.name.clone(),
        None => "nothing".to_string(),
    }
}

// And of a `let .. else`, whose test is an `if` of its own.
fn maybe_else(value: Option<&Item>, wanted: bool) -> &str {
    let chosen = if wanted { value } else { None };
    let Some(item) = chosen else {
        return "nothing";
    };
    &item.name
}

// `v` changes where the `Option` is read: it's what it was.
fn changed(wanted: bool) -> String {
    let before = Item { name: "before".to_string() };
    let later = Item { name: "after".to_string() };
    let mut current = &before;
    if let Some(item) = wanted.then_some(current) {
        current = &later;
        return format!("{} then {}", item.name, current.name);
    }
    format!("none then {}", current.name)
}

// Read past its test, `x` stays what it is.
fn counted(items: &[Item], wanted: bool) -> usize {
    let found = wanted.then_some(items.len());
    let mut n = 0;
    if let Some(len) = found {
        n += len;
    }
    n + found.unwrap_or(10)
}

fn reread(item: &Item, wanted: bool) -> bool {
    let found = wanted.then_some(item);
    if let Some(item) = found {
        println!("read {}", item.name);
    }
    found.is_some()
}

fn main() {
    let item = Item { name: "a".to_string() };
    let items = vec![Item { name: "x".to_string() }, Item { name: "y".to_string() }];
    println!("{} / {}", named(&item, true), named(&item, false));
    println!("{} / {}", first_name(&items, true), first_name(&items, false));
    println!("{} / {} / {}", maybe(Some(&item), true), maybe(None, true), maybe(Some(&item), false));
    println!(
        "{} / {} / {}",
        maybe_else(Some(&item), true),
        maybe_else(None, true),
        maybe_else(Some(&item), false)
    );
    println!("{} / {}", changed(true), changed(false));
    println!("{} / {}", counted(&items, true), counted(&items, false));
    println!("{} / {}", reread(&item, true), reread(&item, false));
    println!("{:?}", wanted_items(&items));
}

fn wanted_items(items: &[Item]) -> Vec<bool> {
    items.iter().map(|item| (item.name == "x").then_some(item).is_some()).collect()
}
