// rd632 (ADR 0861, amended, the user's decision (B)): a `let` that a
// call may write `undefined`, React's `previousMeasurements = state.clones
// ?.map(..)`, read by a nullable parameter of the crate's own, through
// `as_deref()`, and otherwise only by tests that read `null` and
// `undefined` alike, `is_some()` and `map_or`, prints its `None` as `null`
// where it's written, so the parameter is tested `!== null`. Read any
// other way, by a binding's parameter that isn't nullable, returned,
// stored in a field, captured or lent as a `&mut`, it's as it was, and the
// parameter is tested `!= null`. `String(undefined)` is "undefined", which
// the binding's native body says too.
pub struct State {
    pub clones: Option<Vec<u32>>,
}

pub struct Fiber {
    pub memoized: std::cell::Cell<Option<&'static [u32]>>,
}

pub struct Kept {
    pub previous: Option<Vec<u32>>,
}

#[cfg_attr(rust_js, rust_js::link_name = "String")]
fn shown(value: Option<&[u32]>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(value) => value.iter().map(u32::to_string).collect::<Vec<_>>().join(","),
    }
}

fn measured(clones: &[u32]) -> Vec<u32> {
    clones.iter().map(|clone| clone * 10).collect()
}

fn previous_of(state: &State, gesture: bool) -> Option<Vec<u32>> {
    if gesture { None } else { state.clones.clone() }
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measure(previous: Option<&[u32]>) -> u32 {
    match previous {
        Some(previous) => previous.iter().sum(),
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measure_shown(previous: Option<&[u32]>) -> u32 {
    match previous {
        Some(previous) => previous.iter().sum(),
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measure_returned(previous: Option<&[u32]>) -> u32 {
    match previous {
        Some(previous) => previous.iter().sum(),
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measure_stored(previous: Option<&[u32]>) -> u32 {
    match previous {
        Some(previous) => previous.iter().sum(),
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measure_captured(previous: Option<&[u32]>) -> u32 {
    match previous {
        Some(previous) => previous.iter().sum(),
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measure_lent(previous: Option<&[u32]>) -> u32 {
    match previous {
        Some(previous) => previous.iter().sum(),
        None => 0,
    }
}

fn cleared(previous: &mut Option<Vec<u32>>, clear: bool) {
    if clear {
        *previous = None;
    }
}

pub fn update(state: &State, gesture: bool) -> (u32, bool, usize) {
    let previous = if gesture {
        let clones = state.clones.as_ref();
        clones.map(|clones| measured(clones))
    } else {
        let previous = previous_of(state, gesture);
        println!("cleared");
        previous
    };
    let in_viewport = measure(previous.as_deref());
    let some = previous.is_some();
    let count = previous.map_or(0, |previous| previous.len());
    (in_viewport, some, count)
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measure_matched(previous: Option<&[u32]>) -> u32 {
    match previous {
        Some(previous) => previous.iter().sum(),
        None => 0,
    }
}

// Upstream's own shape, `if (clones === null) { previous = null; } else
// { .. }`: its other arm's call may still give `undefined`.
pub fn update_matched(state: &State, gesture: bool) -> (u32, usize) {
    let previous = if gesture {
        match state.clones.as_ref() {
            None => None,
            Some(clones) => Some(measured(clones)),
        }
    } else {
        previous_of(state, gesture)
    };
    let in_viewport = measure_matched(previous.as_deref());
    (in_viewport, previous.map_or(0, |previous| previous.len()))
}

// React's own arm: what a cell held, read, then the cell cleared, `let
// previous = js::cast(old_fiber.memoized_state.get()); old_fiber
// .memoized_state.set(js::null()); previous`.
pub fn update_cleared(state: &State, fiber: &Fiber, gesture: bool) -> u32 {
    let previous = if gesture {
        state.clones.as_ref().map(|clones| measured(clones))
    } else {
        let previous = fiber.memoized.get().map(<[u32]>::to_vec);
        fiber.memoized.set(None);
        previous
    };
    measure_cleared(previous.as_deref()) + previous.map_or(0, |previous| previous.len() as u32)
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measure_cleared(previous: Option<&[u32]>) -> u32 {
    match previous {
        Some(previous) => previous.iter().sum(),
        None => 0,
    }
}

pub fn update_shown(state: &State, gesture: bool) -> (u32, String) {
    let previous = if gesture { state.clones.as_ref().map(|clones| measured(clones)) } else { previous_of(state, gesture) };
    (measure_shown(previous.as_deref()), shown(previous.as_deref()))
}

pub fn update_returned(state: &State, gesture: bool) -> (u32, Option<Vec<u32>>) {
    let previous = if gesture { state.clones.as_ref().map(|clones| measured(clones)) } else { previous_of(state, gesture) };
    (measure_returned(previous.as_deref()), previous)
}

pub fn update_stored(state: &State, gesture: bool) -> (u32, Kept) {
    let previous = if gesture { state.clones.as_ref().map(|clones| measured(clones)) } else { previous_of(state, gesture) };
    let measured = measure_stored(previous.as_deref());
    (measured, Kept { previous })
}

pub fn update_captured(state: &State, gesture: bool) -> u32 {
    let previous = if gesture { state.clones.as_ref().map(|clones| measured(clones)) } else { previous_of(state, gesture) };
    let measured = measure_captured(previous.as_deref());
    let count = move || previous.map_or(0, |previous| previous.len());
    measured + count() as u32
}

pub fn update_lent(state: &State, gesture: bool, clear: bool) -> u32 {
    let mut previous = if gesture { state.clones.as_ref().map(|clones| measured(clones)) } else { previous_of(state, gesture) };
    cleared(&mut previous, clear);
    measure_lent(previous.as_deref())
}

fn main() {
    let none = State { clones: None };
    let some = State {
        clones: Some(vec![1, 2]),
    };
    for state in [&none, &some] {
        for gesture in [true, false] {
            println!("{:?}", update(state, gesture));
            println!("{:?}", update_matched(state, gesture));
            let fiber = Fiber {
                memoized: std::cell::Cell::new(Some(&[4, 5])),
            };
            println!("{} {}", update_cleared(state, &fiber, gesture), update_cleared(state, &fiber, gesture));
            println!("{:?}", update_shown(state, gesture));
            println!("{:?}", update_returned(state, gesture));
            let (measured, kept) = update_stored(state, gesture);
            println!("{measured} {:?}", kept.previous);
            println!("{}", update_captured(state, gesture));
            println!("{} {}", update_lent(state, gesture, false), update_lent(state, gesture, true));
        }
    }
}
