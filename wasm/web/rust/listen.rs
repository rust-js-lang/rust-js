// Listening to the DOM from an effect, whose cleanup stops it.

use webapi::{AbortController, AddEventListenerOptions, Event, EventTarget, EventTargetExt};

/// `target.addEventListener(type, f, { signal })`: until `controller` aborts.
pub fn listen(target: &EventTarget, r#type: &str, f: Box<dyn FnMut(&Event)>, controller: &AbortController) {
    target.add_event_listener_named_with_options(
        r#type,
        f,
        AddEventListenerOptions {
            signal: Some(controller.signal()),
            ..Default::default()
        },
    );
}
