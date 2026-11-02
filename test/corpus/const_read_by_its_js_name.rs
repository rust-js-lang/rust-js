// A constant JS names, `#[rust_js::link_name = "Event.AT_TARGET"]`, as
// webapi's are, of WebIDL's own integer type, is its JS name where it's
// read, `Event.AT_TARGET`, and its value Rust's: a `match` of it compares
// that value, and `as` converts it as Rust does, an integer's widening the
// constant itself (ADRs 0031, 0994, amended, rd618).
#[cfg_attr(rust_js, rust_js::link_name = "Event.NONE")]
const NONE: u16 = 0;
#[cfg_attr(rust_js, rust_js::link_name = "Event.CAPTURING_PHASE")]
const CAPTURING_PHASE: u16 = 1;
#[cfg_attr(rust_js, rust_js::link_name = "Event.AT_TARGET")]
const AT_TARGET: u16 = 2;
#[cfg_attr(rust_js, rust_js::link_name = "Event.BUBBLING_PHASE")]
const BUBBLING_PHASE: u16 = 3;

fn phase_name(phase: u16) -> &'static str {
    match phase {
        NONE => "none",
        CAPTURING_PHASE => "capturing",
        AT_TARGET => "at target",
        BUBBLING_PHASE => "bubbling",
        _ => "unknown",
    }
}

fn flags(position: u32) -> (bool, bool) {
    (
        position & AT_TARGET as u32 != 0,
        position & BUBBLING_PHASE as u32 == BUBBLING_PHASE as u32,
    )
}

fn main() {
    for phase in [NONE, CAPTURING_PHASE, AT_TARGET, BUBBLING_PHASE, 7] {
        println!("{} {}", phase, phase_name(phase));
    }
    println!("{:?} {:?}", flags(2), flags(5));
    let wide = AT_TARGET as f64 / 4.0;
    let sum = u32::from(AT_TARGET) + BUBBLING_PHASE as u32;
    println!("{} {} {}", wide, sum, AT_TARGET as i8 - 3);
}
