// A `#[rust_js::nullable]` field of an `Option` of a value that's never
// `null` but may be `undefined`, an untagged enum's `#[rust_js::undefined]`
// variant or a `()`, upstream's `serverModuleMap: null | ServerManifest`
// where `type ServerManifest = void`: its `None` `null`, its `Some` the
// value itself, `undefined`, tested `!== null` (rd645, the user's (B)).
// Made `Some` and `None`, as a constant, a variable and a parameter;
// tested by `is_some()`, `is_none()`, `match`, `if let` and `matches!`;
// read into a variable that's tested, one given to a function, and one
// set back; cloned, compared, shown and defaulted, against native Rust.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Manifest {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Undefined,
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Default,
    Fast,
}

static MANIFEST: Manifest = Manifest::Undefined;
const GIVEN: Option<&Manifest> = Some(&Manifest::Undefined);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Consumer {
    pub name: String,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub map: Option<&'static Manifest>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub mode: Option<Mode>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub unit: Option<()>,
}

fn tests(consumer: &Consumer) -> String {
    let mut out = String::new();
    if consumer.map.is_some() {
        out.push('s');
    }
    if consumer.map.is_none() {
        out.push('n');
    }
    match consumer.mode {
        None => out.push('-'),
        Some(Mode::Default) => out.push('d'),
        Some(Mode::Fast) => out.push('f'),
    }
    if let Some(()) = consumer.unit {
        out.push('u');
    }
    if matches!(consumer.mode, Some(Mode::Fast)) {
        out.push('!');
    }
    if let Some(map) = consumer.map {
        out.push_str(&format!("{map:?}"));
    }
    out
}

fn present(map: Option<&'static Manifest>) -> bool {
    map.is_some()
}

fn shown(consumer: &Consumer) -> String {
    let map = consumer.map;
    match map {
        Some(map) => format!("{map:?}"),
        None => "-".to_string(),
    }
}

fn kept(consumer: &Consumer) -> (bool, bool) {
    let map = consumer.map;
    let mode = consumer.mode;
    let tested = map.is_some() && mode != Some(Mode::Fast);
    (tested, present(map))
}

fn made(at: u32, mode: Option<Mode>) -> Consumer {
    let map = if at % 2 == 0 { Some(&MANIFEST) } else { None };
    let unit = if at > 1 { Some(()) } else { None };
    let shown = match map {
        Some(map) => format!("{map:?}"),
        None => "-".to_string(),
    };
    Consumer {
        name: format!("c{at}{shown}"),
        map,
        mode,
        unit,
    }
}

fn moved(from: &Consumer, to: &mut Consumer) {
    let map = from.map;
    let mode = from.mode;
    to.map = map;
    to.mode = mode;
    to.unit = from.unit;
}

fn main() {
    let constants = [
        Consumer::default(),
        Consumer {
            name: "some".to_string(),
            map: Some(&Manifest::Undefined),
            mode: Some(Mode::Default),
            unit: Some(()),
        },
        Consumer {
            name: "fast".to_string(),
            map: None,
            mode: Some(Mode::Fast),
            unit: None,
        },
    ];
    for consumer in &constants {
        println!("{} {} {:?}", consumer.name, tests(consumer), kept(consumer));
        println!("{:?}", consumer);
    }
    let modes = [None, Some(Mode::Default), Some(Mode::Fast)];
    for (at, mode) in modes.into_iter().enumerate() {
        let consumer = made(at as u32, mode);
        println!("{} {} {:?}", consumer.name, tests(&consumer), kept(&consumer));
        let copied = consumer.clone();
        println!("{} {}", copied == consumer, tests(&copied));
        let mut into = made(at as u32 + 1, None);
        moved(&consumer, &mut into);
        println!("{} {}", tests(&into), into == consumer);
    }
    let mut changed = made(1, None);
    changed.map = Some(&MANIFEST);
    changed.mode = Some(Mode::Default);
    println!("{}", tests(&changed));
    changed.map = None;
    changed.unit = Some(());
    println!("{} {}", shown(&changed), shown(&constants[1]));
    changed.map = GIVEN;
    let given = Consumer {
        name: "given".to_string(),
        map: GIVEN,
        mode: None,
        unit: None,
    };
    println!("{} {} {}", tests(&changed), tests(&given), shown(&given));
    println!("{} {}", tests(&changed), present(changed.map));
}
