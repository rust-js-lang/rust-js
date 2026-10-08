use std::cell::Cell;

use webapi::{document};
use webapi::events::{Click, Keydown};
use webapi::{AddEventListenerOptions, Event, EventListenerOptions, EventTargetExt, HTMLButtonElement, PointerEvent, abort_controller, listener};

unsafe extern "Rust" {
    #[link_name = "globalThis.record"]
    safe fn record(value: i32);
}

pub fn methods(button: &HTMLButtonElement) {
    button.add_event_listener(Click, |e| record(e.client_x()));
    document.add_event_listener(Keydown, |e| record(if e.key() == "Enter" { 5 } else { 0 }));
}

pub fn mutable(button: &HTMLButtonElement) {
    let mut count = 0;
    button.add_event_listener(Click, move |_| {
        count += 1;
        record(count);
    });
}

pub fn shared_mutable(button: &HTMLButtonElement) {
    let count = Cell::new(0);
    let callback = listener::<PointerEvent>(move |_| {
        count.set(count.get() + 1);
        record(count.get());
    });
    button.add_event_listener(Click, callback);
    button.add_event_listener(Click, callback);
}

pub fn removed(button: &HTMLButtonElement) {
    let listener = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener(Click, listener);
    button.remove_event_listener(Click, listener);
}

pub fn duplicate(button: &HTMLButtonElement) {
    let listener = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener(Click, listener);
    button.add_event_listener(Click, listener);
}

pub fn different(button: &HTMLButtonElement) {
    let first = listener::<PointerEvent>(|e| record(e.client_x()));
    let second = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener(Click, first);
    button.remove_event_listener(Click, second);
}

pub fn once(button: &HTMLButtonElement) {
    button.add_event_listener_with_options(
        Click,
        |e| record(e.client_x()),
        AddEventListenerOptions {
            once: Some(true),
            ..Default::default()
        },
    );
}

pub fn aborted(button: &HTMLButtonElement) {
    let controller = abort_controller::new();
    button.add_event_listener_with_options(
        Click,
        |e| record(e.client_x()),
        AddEventListenerOptions {
            signal: Some(controller.signal()),
            ..Default::default()
        },
    );
    controller.abort();
}

pub fn capture(button: &HTMLButtonElement) {
    let listener = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener_with_options(Click, listener, true);
    // Removal without the capture flag must leave the capture listener installed.
    button.remove_event_listener(Click, listener);
}

pub fn capture_removed(button: &HTMLButtonElement) {
    let listener = listener::<PointerEvent>(|e| record(e.client_x()));
    button.add_event_listener_with_options(Click, listener, true);
    button.remove_event_listener_with_options(Click, listener, true);
}

pub fn named(button: &HTMLButtonElement) {
    button.add_event_listener_named("custom", |e| {
        e.prevent_default();
        record(1);
    });
}
pub fn named_once(button: &HTMLButtonElement) {
    button.add_event_listener_named_with_options(
        "custom",
        |_| record(1),
        AddEventListenerOptions {
            once: Some(true),
            ..Default::default()
        },
    );
}

pub fn named_removed(button: &HTMLButtonElement) {
    let callback = listener::<Event>(|_| record(1));
    button.add_event_listener_named("custom", callback);
    button.remove_event_listener_named("custom", callback);
}

pub fn named_capture_removed(button: &HTMLButtonElement) {
    let callback = listener::<Event>(|_| record(1));
    button.add_event_listener_named_with_options("custom", callback, true);
    button.remove_event_listener_named_with_options("custom", callback, EventListenerOptions { capture: Some(true) });
}
