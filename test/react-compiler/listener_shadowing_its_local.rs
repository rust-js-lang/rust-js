//! #1702, a consumer's: a listener and a local in its closure both named
//! `key`, as Rust writes them.
#![allow(non_snake_case)]
use react::{JSX, jsx, use_effect, use_state};
use webapi::{EventTargetExt, KeyboardEvent, events, window};

pub fn Keys() -> JSX::Element {
    let (last, set_last) = use_state(String::new);
    use_effect(
        move || {
            let key = webapi::listener(move |event: &KeyboardEvent| {
                let key = event.key();
                set_last.set(key);
            });
            window.add_event_listener(events::Keydown, key, None);
        },
        Some(&()),
    );
    jsx! { <p>{last}</p> }
}
