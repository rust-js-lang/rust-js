// An `Option` of an `Option` a variable holds, made and read only where
// it's held, is its payload (ADR 0051, amended): its `None` `undefined`,
// its `Some(None)` `null` and its `Some(Some(v))` `v`, as React's
// `processDispatchQueueItemsInOrder` keeps the fiber of the listener
// before, `previousInstance = instance`, and compares it, `instance !==
// previousInstance`. One given to a function, or of a payload that may
// look like `None` itself, keeps its box.
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Fiber {
    tag: u32,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

struct Listener {
    #[cfg_attr(rust_js, rust_js::nullable)]
    instance: Option<Rc<Fiber>>,
    name: &'static str,
}

struct Plain {
    instance: Option<Rc<Fiber>>,
}

fn dispatched(listeners: &[Listener], stop_at: &str) -> Vec<&'static str> {
    let mut previous_instance = None;
    let mut names = Vec::new();
    for Listener { instance, name } in listeners.iter().rev() {
        if previous_instance != Some(instance) && *name == stop_at {
            return names;
        }
        names.push(*name);
        previous_instance = Some(instance);
    }
    names
}

fn changes(plains: &[Plain]) -> usize {
    let mut previous = None;
    let mut count = 0;
    for Plain { instance } in plains {
        if Some(instance) != previous {
            count += 1;
        }
        previous = Some(instance);
    }
    count
}

fn describe(found: Option<Option<u32>>) -> String {
    format!("{found:?}")
}

fn lookups(values: &[Option<u32>], wanted: usize) -> String {
    let mut last = None;
    let mut seen = 0;
    for (i, value) in values.iter().enumerate() {
        if i == wanted {
            last = Some(*value);
        }
        if matches!(last, Some(None)) {
            seen += 1;
        }
    }
    let shown = match last {
        None => "nothing".to_string(),
        Some(None) => "a none".to_string(),
        Some(Some(n)) => format!("{n}"),
    };
    let inner = if let Some(inner) = last { inner.is_some() } else { false };
    format!("{shown} {seen} {inner}")
}

fn escapes(values: &[Option<u32>]) -> String {
    let mut last = None;
    for value in values {
        last = Some(*value);
    }
    describe(last)
}

#[derive(PartialEq)]
struct Point {
    x: i32,
}

fn points(points: &[Option<Point>]) -> usize {
    let mut previous = None;
    let mut runs = 0;
    for point in points {
        if previous != Some(point) {
            runs += 1;
        }
        previous = Some(point);
    }
    runs
}

fn boxes(boxes: &[Option<Box<u32>>]) -> usize {
    let mut previous = None;
    let mut runs = 0;
    for boxed in boxes {
        if Some(boxed) != previous {
            runs += 1;
        }
        previous = Some(boxed);
    }
    runs
}

fn texts(words: &[Option<String>]) -> usize {
    let mut previous: Option<&Option<String>> = None;
    let mut runs = 0;
    for word in words {
        if previous != Some(word) {
            runs += 1;
        }
        previous = Some(word);
    }
    runs
}

fn main() {
    let a = Rc::new(Fiber { tag: 1 });
    let b = Rc::new(Fiber { tag: 1 });
    let listeners = [
        Listener {
            instance: Some(Rc::clone(&a)),
            name: "a",
        },
        Listener {
            instance: Some(Rc::clone(&a)),
            name: "a2",
        },
        Listener {
            instance: None,
            name: "none2",
        },
        Listener {
            instance: None,
            name: "none",
        },
        Listener {
            instance: Some(Rc::clone(&b)),
            name: "b",
        },
    ];
    println!(
        "{:?} {:?} {:?}",
        dispatched(&listeners, "a2"),
        dispatched(&listeners, "a"),
        dispatched(&listeners, "none2")
    );
    let plains = [
        Plain { instance: None },
        Plain { instance: None },
        Plain {
            instance: Some(Rc::clone(&b)),
        },
        Plain {
            instance: Some(Rc::clone(&a)),
        },
        Plain {
            instance: Some(Rc::clone(&a)),
        },
    ];
    println!("{}", changes(&plains));
    let values = [Some(1), None, Some(3)];
    println!(
        "{} | {} | {} | {}",
        lookups(&values, 0),
        lookups(&values, 1),
        lookups(&values, 2),
        lookups(&values, 9)
    );
    let words = [
        None,
        None,
        Some("a".to_string()),
        Some("a".to_string()),
        Some("b".to_string()),
        None,
    ];
    let at = [
        None,
        None,
        Some(Point { x: 1 }),
        Some(Point { x: 1 }),
        Some(Point { x: 2 }),
        None,
    ];
    let boxed = [None, Some(Box::new(1)), Some(Box::new(1)), None, None];
    println!(
        "{} {} {} {} {}",
        texts(&words),
        points(&at),
        boxes(&boxed),
        escapes(&values[..2]),
        a.tag + b.tag
    );
}
