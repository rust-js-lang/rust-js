// What std lends from inside its storage, in a crate that asks an address
// of something else (#1293): a lend is refused only where an address of
// what it lends may be asked, the lent type as the call's arguments make it
// and normalized, `<PathBuf as Deref>::Target` a `Path`, `<Split<char> as
// Iterator>::Item` a `&str`; of generic code, each instance its callers
// give; `by_ref()` lends its receiver itself, nothing inside it; and a
// shared lend of a value that holds no JS object, a `OnceCell<Option<u8>>`'s,
// a `Peekable`'s `u32`, is a copy no pointer is made of, though the crate
// asks those types' addresses.
use std::cell::OnceCell;
use std::path::{Path, PathBuf};

struct Inner {
    a: u8,
    b: u32,
    maybe: Option<u8>,
}

struct Outer {
    tag: u16,
    inner: Inner,
}

fn extension_of(path: &PathBuf) -> Option<&str> {
    let as_path: &Path = path;
    as_path.extension().and_then(|e| e.to_str())
}

fn words_until_blank(text: &str) -> Vec<&str> {
    let mut words = text.split(' ').peekable();
    let mut kept = Vec::new();
    while let Some(next) = words.peek() {
        if next.is_empty() {
            break;
        }
        kept.push(*next);
        words.next();
    }
    kept
}

fn first_two_then_rest(items: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut all = items.into_iter();
    let first: Vec<String> = all.by_ref().take(2).collect();
    (first, all.collect())
}

fn longer<T: Clone + AsRef<str>>(slot: &mut Result<T, String>, extra: T) -> Option<T> {
    match slot.as_mut() {
        Ok(held) if held.as_ref().len() < extra.as_ref().len() => {
            *held = extra;
            Some(held.clone())
        }
        Ok(_) => None,
        Err(e) => {
            e.push('!');
            None
        }
    }
}

// Generic code's shared lend of what holds no JS object in each instance
// its callers give, a `u32`: a copy, let through by its instances.
fn first_set<T: Copy>(cell: &OnceCell<T>) -> Option<T> {
    cell.get().copied()
}

fn main() {
    let outer = Outer {
        tag: 1,
        inner: Inner { a: 2, b: 3, maybe: Some(4) },
    };
    let base = &raw const outer as usize;
    let field = &raw const outer.inner.b as usize;
    let maybe = &raw const outer.inner.maybe as usize;
    println!("{} {} {} {}", field - base < 16, maybe - base < 16, outer.tag, outer.inner.a);

    let cell: OnceCell<Option<u8>> = OnceCell::new();
    println!("{:?}", cell.get());
    println!("{:?} {:?}", cell.get_or_init(|| Some(9)), cell.get());
    let mut counts = [3u32, 0, 5].into_iter().peekable();
    let mut kept = Vec::new();
    while let Some(&next) = counts.peek() {
        counts.next();
        if next > 0 {
            kept.push(next);
        }
    }
    println!("{kept:?}");
    let mut items = vec![5u32, 6, 7].into_iter();
    let at = &raw const items as usize;
    let firsts: Vec<u32> = items.by_ref().take(2).collect();
    println!("{firsts:?} {:?} {}", items.next(), &raw const items as usize == at);

    println!("{:?} {:?}", extension_of(&PathBuf::from("a/b.txt")), extension_of(&PathBuf::from("c")));
    println!("{:?}", words_until_blank("one two  three"));
    let (first, rest) = first_two_then_rest(vec!["a".into(), "b".into(), "c".into(), "d".into()]);
    println!("{first:?} {rest:?}");

    let parsed: Result<String, String> = Ok(String::from("kept"));
    println!("{:?} {:?}", parsed.as_deref(), Err::<String, &str>("no").as_deref());
    let mut slot: Result<String, String> = Ok(String::from("ab"));
    println!("{:?} {:?}", longer(&mut slot, String::from("abc")), longer(&mut slot, String::from("x")));
    let mut failed: Result<String, String> = Err(String::from("bad"));
    println!("{:?} {failed:?}", longer(&mut failed, String::from("y")));
    failed = Ok(String::from("z"));
    println!("{failed:?}");
    let counted: OnceCell<u32> = OnceCell::new();
    let _ = counted.set(7);
    println!("{:?}", first_set(&counted));
}
