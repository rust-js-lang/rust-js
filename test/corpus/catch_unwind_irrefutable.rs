// `let Err(x) = catch_unwind(f)` of an `f` that always panics, whose `Ok`
// is uninhabited, is JS's `try` of `f`'s body (ADR 0706), and what follows
// it that reads `x`, or what that declares, is its `catch` (ADR 0830):
// each panic caught, its payload bound, what follows run once in Rust's
// order, a `?`, a `continue` and a `break` among it, and what's dropped
// dropped where Rust drops it.
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static PREFIX: RefCell<Option<String>> = const { RefCell::new(None) };
    static SUFFIX: RefCell<String> = const { RefCell::new(String::new()) };
}

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// React's `describeBuiltInComponentFrame`: what's read of the payload,
/// and what that's read for, set once.
fn describe(name: &str) -> String {
    if PREFIX.with_borrow(Option::is_none) {
        // What the first line starts with.
        let Err(x) = catch_unwind(|| panic!("Error\n    at one\n    at two"));
        let stack = x.downcast_ref::<&str>().copied().unwrap_or_default();
        let line = stack.lines().nth(1).unwrap_or_default();
        PREFIX.set(Some(line.trim_end_matches("one").to_string()));
        SUFFIX.set(if stack.contains("\n    at") { " (<anonymous>)" } else { "" }.to_string());
    }
    let prefix = PREFIX.with_borrow(|prefix| prefix.clone().unwrap_or_default());
    let suffix = SUFFIX.with_borrow(String::clone);
    format!("{prefix}{name}{suffix}")
}

/// React's `DetermineComponentFrameRoot`: the control set, then what
/// follows, which reads neither it nor the payload, after the `catch`.
fn root(n: u32) -> Option<(String, String)> {
    let mut control = None;
    if let Err(sample) = catch_unwind(AssertUnwindSafe(|| {
        let Err(x) = catch_unwind(|| panic!("control"));
        control = x.downcast_ref::<&str>().copied();
        println!("rendering {n}");
        if n > 1 {
            panic!("sample");
        }
    })) && let Some(sample) = sample.downcast_ref::<&str>()
        && let Some(control) = control
    {
        return Some((sample.to_string(), control.to_string()));
    }
    None
}

/// What reads what read the payload is in the `catch`, what doesn't after
/// it, and a name shadowed after it is the shadow.
fn chain(k: usize) -> String {
    let Err(x) = catch_unwind(|| panic!("abcdef"));
    let text = x.downcast_ref::<&str>().copied().unwrap_or_default();
    println!("between");
    let head = &text[..k];
    let shout = head.to_uppercase();
    println!("shout {shout}");
    let after = k * 2;
    println!("after {after}");
    let shout = after + 1;
    format!("{shout}")
}

/// The payload's own name shadowed in what follows: the shadow, set in
/// the `catch`.
fn renamed(k: usize) -> String {
    let Err(x) = catch_unwind(|| panic!("abc"));
    let text = x.downcast_ref::<&str>().copied().unwrap_or_default();
    let n = text.len() + k;
    println!("renamed {n}");
    let x = k * 3;
    let n = x + 1;
    format!("{n}")
}

/// A function declared in what follows and called before it, which JS
/// hoists, where it's declared.
fn hoisted(n: u32) -> usize {
    let first = twice(n);
    let Err(x) = catch_unwind(|| panic!("hoisted"));
    fn twice(n: u32) -> u32 {
        n * 2
    }
    let len = x.downcast_ref::<&str>().map_or(0, |m| m.len());
    len + first as usize + twice(1) as usize
}

/// A `?` in what follows returns from the `catch`.
fn parsed(text: &'static str) -> Option<u32> {
    let Err(x) = catch_unwind(move || panic!("{text}"));
    let message = x.downcast_ref::<String>()?;
    let n: u32 = message.parse().ok()?;
    println!("parsed {n}");
    Some(n + 1)
}

/// What's dropped is dropped as Rust drops it: a guard made before, one
/// made in what follows, a return between.
fn dropped(early: bool) -> usize {
    let _outer = Noisy("outer");
    let Err(x) = catch_unwind(|| panic!("dropped"));
    let _guard = Noisy("guard");
    let len = x.downcast_ref::<&str>().map_or(0, |m| m.len());
    println!("len {len}");
    if early {
        return 0;
    }
    println!("late");
    len
}

/// In a loop: a `continue` and a `break` among what follows.
fn looped() -> Vec<usize> {
    let mut lens = Vec::new();
    for i in 0..4 {
        let Err(x) = catch_unwind(|| panic!("loop"));
        if i == 1 {
            continue;
        }
        let len = x.downcast_ref::<&str>().map_or(0, |m| m.len());
        if i == 3 {
            break;
        }
        lens.push(len + i);
    }
    lens
}

/// Its value what follows gives.
fn tail() -> usize {
    let Err(x) = catch_unwind(|| panic!("tail"));
    let message = x.downcast_ref::<&str>().copied();
    message.map_or(0, str::len)
}

/// A function that may return, whose `Ok` isn't uninhabited: an `if let`,
/// what follows it run either way.
fn fake(n: u32) -> u32 {
    if n > 1 {
        panic!("fake {n}");
    }
    n
}

fn maybe(n: u32) -> String {
    let mut seen = String::from("none");
    if let Err(x) = catch_unwind(|| fake(n)) {
        seen = x.downcast_ref::<String>().cloned().unwrap_or_default();
    }
    println!("maybe {n}");
    seen
}

fn main() {
    println!("{}", describe("App"));
    println!("{}", describe("Again"));
    for n in [1, 2] {
        println!("root {n} {:?}", root(n));
    }
    for k in [0, 2, 6] {
        println!("chain {k} {}", chain(k));
    }
    println!("renamed {}", renamed(4));
    println!("hoisted {}", hoisted(5));
    println!("parsed {:?}", parsed("12"));
    println!("parsed {:?}", parsed("x"));
    for early in [false, true] {
        println!("dropped {early} {}", dropped(early));
    }
    println!("looped {:?}", looped());
    println!("tail {}", tail());
    for n in [1, 2] {
        println!("{}", maybe(n));
    }
}
