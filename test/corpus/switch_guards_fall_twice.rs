// A `match` of strings with two guarded arms that fall to the next, and
// arms of names a guard compares with thread-locals, `||` joining them:
// React DOM's SimpleEventPlugin, `"keypress" if .. => return`, `"click" if
// .. => return`, then `name if ANIMATION_END.with(..) || ..`, upstream's
// `switch` whose two guarded cases fall through and whose `case
// ANIMATION_END: case ANIMATION_START:` are stacked (rd118, ADR 0905 and
// 0629 amended). Each guard runs where Rust runs it, once its constant
// matched, and only until one holds.
use std::cell::RefCell;

thread_local! {
    static ANIMATION_END: String = String::from("animationend");
    static ANIMATION_START: RefCell<String> = RefCell::new(String::from("animationstart"));
    static TRANSITION_END: String = String::from("transitionend");
}

fn on(log: &RefCell<Vec<String>>, name: &str, holds: bool) -> bool {
    log.borrow_mut().push(format!("{name}:{holds}"));
    holds
}

fn ctor(name: &str, n: i32, log: &RefCell<Vec<String>>) -> i32 {
    let mut ctor = 0;
    match name {
        // Remove unwanted keypresses.
        "keypress" if on(log, "code", n == 1) => return -1,
        "keypress" | "keydown" | "keyup" => ctor = 1,
        "focusin" => ctor = 2,
        // Remove right clicks.
        "click" if on(log, "button", n == 2) => return -2,
        "click" | "auxclick" | "dblclick" => ctor = 5,
        name if ANIMATION_END.with(|event_name| event_name == name)
            || ANIMATION_START.with_borrow(|event_name| event_name == name) =>
        {
            ctor = 6;
        }
        name if TRANSITION_END.with(|event_name| event_name == name) => ctor = 7,
        "scroll" | "scrollend" => ctor = 3,
        _ => {
            // Unknown event.
        }
    }
    ctor + n
}

fn main() {
    let log = RefCell::new(Vec::new());
    let names = [
        "keypress",
        "keyup",
        "focusin",
        "click",
        "dblclick",
        "animationend",
        "animationstart",
        "transitionend",
        "scroll",
        "other",
    ];
    for name in names {
        println!("{name} {} {} {}", ctor(name, 1, &log), ctor(name, 2, &log), ctor(name, 3, &log));
    }
    ANIMATION_START.with_borrow_mut(|start| *start = String::from("click"));
    for name in ["click", "animationstart"] {
        println!("{name} {} {}", ctor(name, 2, &log), ctor(name, 3, &log));
    }
    ANIMATION_START.with_borrow_mut(|start| *start = String::from("transitionend"));
    println!("transitionend {}", ctor("transitionend", 1, &log));
    println!("{:?}", log.borrow());
}
