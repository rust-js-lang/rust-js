use std::cell::Cell;

use webapi::{document, window};
use webapi::events::{Click, Keydown};
use js::{PropertyDescriptor, function, object};
use webapi::{
    AddEventListenerOptions, Event, EventListenerObject, EventListenerOptions, EventListenerOrEventListenerObject, EventTargetExt,
    EventOrStr, HTMLButtonElement, HTMLImageElement, HTMLLinkElement,
    KeyboardEvent, PointerEvent, abort_controller, listener,
};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
fn record(value: impl Into<f64>) {
    unreachable!()
}

pub fn methods(button: &HTMLButtonElement) {
    button.add_event_listener(Click, |e: &PointerEvent| record(e.client_x()), None);
    document.add_event_listener(Keydown, |e: &KeyboardEvent| record(if e.key() == "Enter" { 5 } else { 0 }), None);
}

pub fn mutable(button: &HTMLButtonElement) {
    let mut count = 0;
    button.add_event_listener(Click, move |_: &PointerEvent| {
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
        |e: &PointerEvent| record(e.client_x()),
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
        |e: &PointerEvent| record(e.client_x()),
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
    button.add_event_listener_named("custom", |e: &Event| {
        e.prevent_default();
        record(1);
    }, None);
}
pub fn named_once(button: &HTMLButtonElement) {
    button.add_event_listener_named(
        "custom",
        |_: &Event| record(1),
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

// A listener JS gave, a function or an object JS calls `handleEvent` of, is
// passed as it is, so removal finds it.
pub fn given(button: &HTMLButtonElement, given: EventListenerOrEventListenerObject) {
    button.add_event_listener_named("custom", given, None);
}

pub fn given_removed(button: &HTMLButtonElement, given: EventListenerOrEventListenerObject) {
    button.add_event_listener_named("custom", given, None);
    button.remove_event_listener_named("custom", given, None);
}

// As upstream React's fragment-scoped `once` calls what it wraps: a function
// with the wrapper's `this`, the target, or an object's `handleEvent`.
pub fn given_wrapped(button: &HTMLButtonElement, given: EventListenerOrEventListenerObject) {
    let attached = function::with_this(move |this, (event,): (&Event,)| {
        record(10);
        match given {
            EventListenerOrEventListenerObject::EventListener(f) => {
                function::call(f, this.map(|t| t as &dyn js::IntoUnknown), (event,));
            }
            EventListenerOrEventListenerObject::EventListenerObject(o) => o.handle_event(event),
        }
    });
    button.add_event_listener_named("custom", attached, None);
}

// FragmentInstance's `once` wrapper, kept to add to each child and remove
// by identity: a function of its `this`, a JS-owned listener (rd209).
pub fn kept_wrapped(buttons: &[&HTMLButtonElement], given: EventListenerOrEventListenerObject) -> EventListenerOrEventListenerObject {
    let attached = listener(function::with_this(move |this, (event,): (&'static Event,)| {
        record(20);
        if let EventListenerOrEventListenerObject::EventListener(f) = given {
            function::call(f, this.map(|t| t as &dyn js::IntoUnknown), (event,));
        }
    }));
    let attached = EventListenerOrEventListenerObject::EventListener(attached);
    for button in buttons {
        button.add_event_listener_named("custom", attached, None);
    }
    attached
}

// React DOM's checkPassiveEvents: an options object whose `passive` is a
// getter, given as the listener too, an `EventListenerObject` of no
// `handleEvent`, which JS calls only where "test" is dispatched; and to
// removal, an `AddEventListenerOptions` an `EventListenerOptions` is.
pub fn passive(button: &HTMLButtonElement) {
    let options = AddEventListenerOptions::default();
    object::define_property(
        &options,
        "passive",
        &PropertyDescriptor {
            get: Some(&|| {
                record(1);
                None
            }),
            ..Default::default()
        },
    );
    // Safety: nothing dispatches "test", so JS never calls its `handleEvent`.
    let listener: &EventListenerObject = unsafe { js::cast(Some(js::unknown(&options))) };
    button.add_event_listener_named("test", listener, Some(&options));
    button.remove_event_listener_named("test", listener, Some(&options));
}

// An event handler property is set to a Rust function or closure, or to any
// JS function value as it is: React DOM's preloadStylesheet keeps a promise
// on a link, which its load fulfils and its error rejects, each with the
// event (rd192).
pub fn settled(link: &'static HTMLLinkElement) {
    js::set(
        link,
        "_p",
        js::promise::new(|resolve: &'static dyn Fn(&'static Event), reject| {
            link.set_onload(Some(&resolve));
            link.set_onerror(Some(&reject));
        }),
    );
}

// An element's `onerror` is called with the event alone; a window's with
// the error's message, where it was and what was thrown.
pub fn errors(image: &'static HTMLImageElement) {
    image.set_onerror(Some(&|e: &Event| record(if e.r#type() == "error" { 7 } else { 0 })));
    window.set_onerror(Some(&|message: EventOrStr<'_>, _: Option<&str>, line: Option<f64>, _: Option<f64>, _: Option<&js::Unknown>| {
        record(if matches!(message, EventOrStr::Str(_)) { line.unwrap_or(0.0) } else { -1.0 });
        true
    }));
}

// A window's `onerror` read and set back is the function it was.
pub fn restored() {
    let saved = window.onerror();
    window.set_onerror(None);
    if let Some(saved) = saved {
        window.set_onerror(Some(&saved));
    }
}
