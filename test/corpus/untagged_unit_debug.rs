// `{:?}` of an untagged enum's variant without fields is its Rust name, as
// a derived `Debug` shows it: of one that's `undefined` or `null`
// (`#[rust_js::undefined]`, `#[rust_js::null]`, ADR 0676) too, which no
// table of names is keyed by, told by `===` first (#510). Of each alone,
// in an `Option`, in a `Vec`, and of an enum with payloads beside them,
// against native Rust.

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Default,
    #[cfg_attr(rust_js, rust_js::null)]
    Cleared,
    Fast,
    #[cfg_attr(rust_js, rust_js::name = "blocking")]
    Blocking,
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Manifest {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Undefined,
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug, Clone, PartialEq)]
enum Slot {
    Items(Vec<i32>),
    #[cfg_attr(rust_js, rust_js::undefined)]
    Missing,
    #[cfg_attr(rust_js, rust_js::null)]
    Cleared,
}

fn pick(at: usize) -> Mode {
    [Mode::Default, Mode::Cleared, Mode::Fast, Mode::Blocking][at % 4]
}

fn main() {
    for at in 0..4 {
        let mode = pick(at);
        println!("{mode:?} {:?} {mode:#?}", Some(mode));
    }
    let modes: Vec<Mode> = (0..4).map(pick).collect();
    println!("{modes:?}");
    println!("{:?} {:?}", Manifest::Undefined, Some(Manifest::Undefined));
    let slots = [Slot::Items(vec![1, 2]), Slot::Missing, Slot::Cleared];
    for slot in &slots {
        println!("{slot:?}");
    }
}
