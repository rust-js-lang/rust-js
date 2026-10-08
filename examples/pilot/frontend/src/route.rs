//! Where the app is: its URL's hash, `#/contacts/2`, followed as it changes.

use react::{use_effect, use_state};
use webapi::events::Hashchange;
use webapi::window;
use webapi::{AddEventListenerOptions, EventTargetExt, abort_controller};

#[derive(Clone, Copy, PartialEq)]
pub enum Route {
    List,
    Contact(u32),
    New,
    NotFound,
}

pub fn parse(hash: &str) -> Route {
    match hash.strip_prefix('#').unwrap_or(hash) {
        "" | "/" => Route::List,
        "/new" => Route::New,
        path => match path.strip_prefix("/contacts/").and_then(|id| id.parse().ok()) {
            Some(id) => Route::Contact(id),
            None => Route::NotFound,
        },
    }
}

fn hash() -> String {
    window.location().hash()
}

/// Go to `hash`, as a link would.
pub fn go(to: &str) {
    window.location().set_hash(to);
}

/// The route, and a render each time it changes.
pub fn use_route() -> Route {
    let (current, set_current) = use_state(hash());
    use_effect(
        move || {
            let controller = abort_controller::new();
            // It's removed when the effect is cleaned up: its signal aborts.
            let options = AddEventListenerOptions {
                signal: Some(controller.signal()),
                ..Default::default()
            };
            window.add_event_listener_with_options(Hashchange, move |_| set_current.set(hash()), options);
            move || controller.abort()
        },
        (),
    );
    parse(current)
}
