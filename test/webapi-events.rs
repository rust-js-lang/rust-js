use std::cell::Cell;

use webapi::{document};
use webapi::events::{Click, Keydown};
use webapi::{AddEventListenerOptions, Event, EventListenerOptions, EventTargetExt, HTMLButtonElement, PointerEvent, abort_controller, listener};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
fn record(value: impl Into<f64>) {
    unreachable!()
}

pub fn methods(button: &HTMLButtonElement) {
    button.add_event_listener(Click, |e| record(e.client_x()), None);
    document.add_event_listener(Keydown, |e| record(if e.key() == "Enter" { 5 } else { 0 }), None);
}

pub fn mutable(button: &HTMLButtonElement) {
    let mut count = 0;
    button.add_event_listener(Click, move |_| {
        count += 1;
        record(count);
    }, None);
}

pub fn shared_mutable(button: &HTMLButtonElement) {
    let count = Cell::new(0);
    let callback = listener::<PointerEvent>(move |_| {
        count.set(count.get() + 1);
        record(count.get());
    });
    button.add_event_listener(Click, callback, None);
    button.add_event_listener(Click, callback, None);
}

pub fn removed(button: &HTMLButtonElement) {
    let listener = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener(Click, listener, None);
    button.remove_event_listener(Click, listener, None);
}

pub fn duplicate(button: &HTMLButtonElement) {
    let listener = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener(Click, listener, None);
    button.add_event_listener(Click, listener, None);
}

pub fn different(button: &HTMLButtonElement) {
    let first = listener::<PointerEvent>(|e| record(e.client_x()));
    let second = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener(Click, first, None);
    button.remove_event_listener(Click, second, None);
}

pub fn once(button: &HTMLButtonElement) {
    button.add_event_listener(
        Click,
        |e| record(e.client_x()),
        Some(&AddEventListenerOptions {
            once: Some(true),
            ..Default::default()
        }),
    );
}

pub fn aborted(button: &HTMLButtonElement) {
    let controller = abort_controller::new();
    button.add_event_listener(
        Click,
        |e| record(e.client_x()),
        Some(&AddEventListenerOptions {
            signal: Some(controller.signal()),
            ..Default::default()
        }),
    );
    controller.abort(None);
}

pub fn capture(button: &HTMLButtonElement) {
    let listener = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener(Click, listener, Some(&true));
    // Removal without the capture flag must leave the capture listener installed.
    button.remove_event_listener(Click, listener, None);
}

pub fn capture_removed(button: &HTMLButtonElement) {
    let listener = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener(Click, listener, Some(&true));
    button.remove_event_listener(Click, listener, Some(&true));
}

pub fn named(button: &HTMLButtonElement) {
    button.add_event_listener_named("custom", |e| {
        e.prevent_default();
        record(1);
    }, None);
}
pub fn named_once(button: &HTMLButtonElement) {
    button.add_event_listener_named(
        "custom",
        |_| record(1),
        Some(&AddEventListenerOptions {
            once: Some(true),
            ..Default::default()
        }),
    );
}

pub fn named_removed(button: &HTMLButtonElement) {
    let callback = listener::<Event>(|_| record(1));
    button.add_event_listener_named("custom", callback, None);
    button.remove_event_listener_named("custom", callback, None);
}

pub fn named_capture_removed(button: &HTMLButtonElement) {
    let callback = listener::<Event>(|_| record(1));
    button.add_event_listener_named("custom", callback, Some(&true));
    button.remove_event_listener_named("custom", callback, Some(&EventListenerOptions { capture: Some(true) }));
}
