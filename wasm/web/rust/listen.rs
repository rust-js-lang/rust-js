// Listening to the DOM from an effect, whose cleanup stops it.

use webapi::{AbortController, AddEventListenerOptions, Event, EventTarget, EventTargetExt, abort_controller};

/// `target.addEventListener(type, f, { signal })`: until `controller` aborts.
pub fn listen(target: &EventTarget, type_: &str, f: Box<dyn FnMut(&Event)>, controller: &AbortController) {
    target.add_event_listener_named_with_options(
        type_,
        f,
        AddEventListenerOptions {
            signal: Some(abort_controller::signal(controller)),
            ..Default::default()
        },
    );
}
