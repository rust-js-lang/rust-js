// What JS made, read through control flow: a getter's value shadowed, held
// across a call that changes what the getter answers, tested in an `else
// if` chain, a loop's test, an early return and `?`, mapped, and an array
// of JS-made objects searched.
//@ call: shadowed [{"count": 3, "bump": {"$fn": null}}]
//@ call: held [{"count": 3, "bump": {"$fn": null}}]
//@ call: graded [{"count": 0}]
//@ call: graded [{"count": 5}]
//@ call: graded [{"count": 50}]
//@ call: counted [{"count": 2, "bump": {"$fn": null}, "done": true}]
//@ call: firstLabel [{"props": {"label": "a"}}]
//@ call: firstLabel [{"props": {}}]
//@ call: firstLabel [{}]
//@ call: lengths [{"props": {"label": "abc"}}]
//@ call: named [[{"name": "a", "count": 1}, {"name": "b", "count": 2}, {"name": "c", "count": 3}]]

use js::Unknown;

pub trait Counter {
    #[cfg_attr(rust_js, rust_js::link_name = "get count")]
    fn count(&self) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get done")]
    fn done(&self) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "bump")]
    fn bump(&self) {
        unreachable!()
    }
}

pub struct Counted;

impl Counter for Counted {}

pub fn shadowed(counter: &Counted) -> u32 {
    let count = counter.count();
    let count = count + 1;
    counter.bump();
    count * 2
}

pub fn held(counter: &Counted) -> u32 {
    let before = counter.count();
    counter.bump();
    let after = counter.count();
    before + after
}

pub fn graded(counter: &Counted) -> &'static str {
    let count = counter.count();
    if count == 0 {
        "none"
    } else if count < 10 {
        "few"
    } else {
        "many"
    }
}

pub fn counted(counter: &Counted) -> u32 {
    let mut turns = 0;
    while !counter.done() && turns < 3 {
        counter.bump();
        turns += 1;
    }
    turns + counter.count()
}

fn label(element: &Unknown) -> Option<String> {
    let props = js::get(element, "props")?;
    let label = js::get(props, "label")?;
    Some(js::string(Some(label)))
}

pub fn first_label(element: &Unknown) -> String {
    match label(element) {
        Some(label) if !label.is_empty() => label,
        Some(_) => "empty".to_string(),
        None => "none".to_string(),
    }
}

pub fn lengths(element: &Unknown) -> usize {
    label(element).map(|label| label.len()).unwrap_or(0) + label(element).map_or(0, |label| label.len())
}

pub struct Named {
    pub name: String,
    pub count: u32,
}

// Safety: what JS gives is an object of these fields.
unsafe impl js::Defined for Named {}

pub fn named(items: &Unknown) -> String {
    // Safety: what's given is an array of them.
    let items: Vec<&Named> = unsafe { js::cast(Some(items)) };
    match items.iter().find(|item| item.count > 1) {
        Some(item) => format!("{} {}", item.name, items.iter().filter(|item| item.name != "c").count()),
        None => String::new(),
    }
}
