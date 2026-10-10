//! [`events`](https://nodejs.org/api/events.html): an [`EventEmitter`], and
//! each of Node's emitters, are typed by their events (ADR 0361): an event
//! is a name of its own, [`event::Exit`](crate::event::Exit) of `"exit"`,
//! and its emitter says its listener's type, [`Emits`].
//!
//! ```rust,ignore
//! #[rust_js::name = "ping"]
//! pub struct Ping;
//!
//! impl Emits<Ping> for EventEmitter {
//!     type Listener = dyn Fn(f64);
//!     type Args = (f64,);
//! }
//!
//! emitter.on(Ping, Box::new(|n| println!("{n}")));
//! emitter.emit(Ping, (1.0,));
//! ```

use core::marker::PhantomData;

use js::{JsObject, Promise, Symbol};

/// What an emitter's event `E` is: its listener's type, `dyn Fn(f64)`, and
/// its arguments as [`EventEmitterExt::emit`] gives them, `(f64,)`. An
/// emitter has each of its events'; a program adds its own to an
/// [`EventEmitter`].
pub trait Emits<E> {
    type Listener: ?Sized;
    type Args;
}

/// [`EventEmitter`](https://nodejs.org/api/events.html#class-eventemitter):
/// what emits events, and calls their listeners.
#[cfg_attr(rust_js, rust_js::types = "EventEmitter")]
pub struct EventEmitter(PhantomData<JsObject>);

pub mod event_emitter {
    use super::{EventEmitter, EventEmitterOptions};

    unsafe extern "Rust" {
        /// [`new EventEmitter()`](https://nodejs.org/api/events.html#class-eventemitter).
        #[link_name = "new events#EventEmitter"]
        pub safe fn new() -> &'static EventEmitter;

        #[link_name = "new events#EventEmitter"]
        pub safe fn new_with_options(options: EventEmitterOptions) -> &'static EventEmitter;
    }
}

impl EventEmitterExt for EventEmitter {}

/// An emitter's methods, of the events it [`Emits`]: an [`EventEmitter`]'s,
/// and of each of Node's that extends one.
pub trait EventEmitterExt: Sized {
    /// [`emitter.on(event, listener)`](https://nodejs.org/api/events.html#emitteroneventname-listener):
    /// call `listener` at each `event`.
    #[cfg_attr(rust_js, rust_js::link_name = "on")]
    fn on<E>(&self, event: E, listener: Box<<Self as Emits<E>>::Listener>) -> &Self
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.addListener(event, listener)`](https://nodejs.org/api/events.html#emitteraddlistenereventname-listener):
    /// [`on`](EventEmitterExt::on), by another name.
    #[cfg_attr(rust_js, rust_js::link_name = "addListener")]
    fn add_listener<E>(&self, event: E, listener: Box<<Self as Emits<E>>::Listener>) -> &Self
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.once(event, listener)`](https://nodejs.org/api/events.html#emitteronceeventname-listener):
    /// call `listener` at the next `event` only.
    #[cfg_attr(rust_js, rust_js::link_name = "once")]
    fn once<E>(&self, event: E, listener: Box<<Self as Emits<E>>::Listener>) -> &Self
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.prependListener(event, listener)`](https://nodejs.org/api/events.html#emitterprependlistenereventname-listener):
    /// call `listener` at each `event`, before the others.
    #[cfg_attr(rust_js, rust_js::link_name = "prependListener")]
    fn prepend_listener<E>(&self, event: E, listener: Box<<Self as Emits<E>>::Listener>) -> &Self
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.prependOnceListener(event, listener)`](https://nodejs.org/api/events.html#emitterprependoncelistenereventname-listener).
    #[cfg_attr(rust_js, rust_js::link_name = "prependOnceListener")]
    fn prepend_once_listener<E>(&self, event: E, listener: Box<<Self as Emits<E>>::Listener>) -> &Self
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.off(event, listener)`](https://nodejs.org/api/events.html#emitteroffeventname-listener):
    /// call `listener` no more, one [`listener`] made to be given here too.
    #[cfg_attr(rust_js, rust_js::link_name = "off")]
    fn off<E>(&self, event: E, listener: &'static <Self as Emits<E>>::Listener) -> &Self
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.removeListener(event, listener)`](https://nodejs.org/api/events.html#emitterremovelistenereventname-listener):
    /// [`off`](EventEmitterExt::off), by another name.
    #[cfg_attr(rust_js, rust_js::link_name = "removeListener")]
    fn remove_listener<E>(&self, event: E, listener: &'static <Self as Emits<E>>::Listener) -> &Self
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.removeAllListeners()`](https://nodejs.org/api/events.html#emitterremovealllistenerseventname):
    /// each event's.
    #[cfg_attr(rust_js, rust_js::link_name = "removeAllListeners")]
    fn remove_all_listeners(&self) -> &Self {
        unreachable!()
    }

    /// `emitter.removeAllListeners(event)`: `event`'s.
    #[cfg_attr(rust_js, rust_js::link_name = "removeAllListeners")]
    fn remove_all_listeners_with_event_name<E>(&self, event: E) -> &Self
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.emit(event, ...args)`](https://nodejs.org/api/events.html#emitteremiteventname-args):
    /// call `event`'s listeners with `args`, and give whether it has any.
    #[cfg_attr(rust_js, rust_js::link_name = "emit")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    fn emit<E>(&self, event: E, args: <Self as Emits<E>>::Args) -> bool
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.listenerCount(event)`](https://nodejs.org/api/events.html#emitterlistenercounteventname-listener).
    #[cfg_attr(rust_js, rust_js::link_name = "listenerCount")]
    fn listener_count<E>(&self, event: E) -> f64
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// `emitter.listenerCount(event, listener)`: how often `listener` is one.
    #[cfg_attr(rust_js, rust_js::link_name = "listenerCount")]
    fn listener_count_with_listener<E>(&self, event: E, listener: &'static <Self as Emits<E>>::Listener) -> f64
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.listeners(event)`](https://nodejs.org/api/events.html#emitterlistenerseventname).
    #[cfg_attr(rust_js, rust_js::link_name = "listeners")]
    fn listeners<E>(&self, event: E) -> Vec<&'static <Self as Emits<E>>::Listener>
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.rawListeners(event)`](https://nodejs.org/api/events.html#emitterrawlistenerseventname):
    /// its listeners, a `once`'s as its wrapper.
    #[cfg_attr(rust_js, rust_js::link_name = "rawListeners")]
    fn raw_listeners<E>(&self, event: E) -> Vec<&'static <Self as Emits<E>>::Listener>
    where
        Self: Emits<E>,
    {
        unreachable!()
    }

    /// [`emitter.eventNames()`](https://nodejs.org/api/events.html#emittereventnames):
    /// the events it has listeners of.
    #[cfg_attr(rust_js, rust_js::link_name = "eventNames")]
    fn event_names(&self) -> Vec<String> {
        unreachable!()
    }

    /// [`emitter.getMaxListeners()`](https://nodejs.org/api/events.html#emittergetmaxlisteners).
    #[cfg_attr(rust_js, rust_js::link_name = "getMaxListeners")]
    fn get_max_listeners(&self) -> f64 {
        unreachable!()
    }

    /// [`emitter.setMaxListeners(n)`](https://nodejs.org/api/events.html#emittersetmaxlistenersn):
    /// warn of a leak beyond `n` of an event's listeners, 0 never.
    #[cfg_attr(rust_js, rust_js::link_name = "setMaxListeners")]
    fn set_max_listeners(&self, n: f64) -> &Self {
        unreachable!()
    }
}

/// A listener, made to be given to [`EventEmitterExt::on`], `Box::new(it)`,
/// and to [`EventEmitterExt::off`] too: the same function.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
pub fn listener<L: ?Sized>(this: Box<L>) -> &'static L {
    unreachable!()
}

/// [`events.once(emitter, event)`](https://nodejs.org/api/events.html#eventsonceemitter-name-options):
/// `event`'s next arguments.
#[cfg_attr(rust_js, rust_js::link_name = "events#once")]
pub fn once<T: Emits<E>, E>(emitter: &T, event: E) -> Promise<<T as Emits<E>>::Args> {
    unreachable!()
}

/// `events.once(emitter, event, options)`: rejected where `options`'
/// signal aborts first.
#[cfg_attr(rust_js, rust_js::link_name = "events#once")]
pub fn once_with_options<T: Emits<E>, E>(
    emitter: &T,
    event: E,
    options: StaticEventEmitterOptions<'_>,
) -> Promise<<T as Emits<E>>::Args> {
    unreachable!()
}

/// [`events.listenerCount(emitter, event)`](https://nodejs.org/api/events.html#eventslistenercountemitter-eventname).
/// Deprecated: [`EventEmitterExt::listener_count`] is the emitter's.
#[cfg_attr(rust_js, rust_js::link_name = "events#listenerCount")]
pub fn listener_count<T: Emits<E>, E>(emitter: &T, event: E) -> f64 {
    unreachable!()
}

/// [`events.getEventListeners(emitter, event)`](https://nodejs.org/api/events.html#eventsgeteventlistenersemitterortarget-eventname).
#[cfg_attr(rust_js, rust_js::link_name = "events#getEventListeners")]
pub fn get_event_listeners<T: Emits<E>, E>(emitter: &T, event: E) -> Vec<&'static <T as Emits<E>>::Listener> {
    unreachable!()
}

/// [`events.getMaxListeners(emitter)`](https://nodejs.org/api/events.html#eventsgetmaxlistenersemitterortarget).
#[cfg_attr(rust_js, rust_js::link_name = "events#getMaxListeners")]
pub fn get_max_listeners<T: EventEmitterExt>(emitter: &T) -> f64 {
    unreachable!()
}

/// [`events.setMaxListeners(n)`](https://nodejs.org/api/events.html#eventssetmaxlistenersn-eventtargets):
/// each new emitter's.
#[cfg_attr(rust_js, rust_js::link_name = "events#setMaxListeners")]
pub fn set_max_listeners(n: f64) {
    unreachable!()
}

/// `events.setMaxListeners(n, ...eventTargets)`: each of `event_targets`'.
#[cfg_attr(rust_js, rust_js::link_name = "events#setMaxListeners")]
#[cfg_attr(rust_js, rust_js::variadic)]
pub fn set_max_listeners_with_event_targets(n: f64, event_targets: &[&EventEmitter]) {
    unreachable!()
}

/// [`events.addAbortListener(signal, listener)`](https://nodejs.org/api/events.html#eventsaddabortlistenersignal-listener):
/// call `resource` once `signal` aborts, as no other listener stops; what
/// it gives, disposed, removes it.
#[cfg_attr(rust_js, rust_js::link_name = "events#addAbortListener")]
pub fn add_abort_listener(signal: &webapi::AbortSignal, resource: impl Fn(&webapi::Event) + 'static) -> &'static JsObject {
    unreachable!()
}

unsafe extern "Rust" {
    /// [`events.errorMonitor`](https://nodejs.org/api/events.html#eventserrormonitor):
    /// the event of an `"error"`, before its listeners.
    #[link_name = "events#errorMonitor"]
    #[allow(non_upper_case_globals)]
    pub safe static error_monitor: &'static Symbol;

    /// [`events.captureRejectionSymbol`](https://nodejs.org/api/events.html#eventscapturerejectionsymbol):
    /// the method an emitter's rejected listener's promise calls.
    #[link_name = "events#captureRejectionSymbol"]
    #[allow(non_upper_case_globals)]
    pub safe static capture_rejection_symbol: &'static Symbol;

    /// [`events.captureRejections`](https://nodejs.org/api/events.html#eventscapturerejections):
    /// whether a new emitter's listener's rejected promise is its `"error"`.
    #[link_name = "get events#EventEmitter.captureRejections"]
    pub safe fn capture_rejections() -> bool;

    #[link_name = "set events#EventEmitter.captureRejections"]
    pub safe fn set_capture_rejections(value: bool);

    /// [`events.defaultMaxListeners`](https://nodejs.org/api/events.html#eventsdefaultmaxlisteners):
    /// a new emitter's most listeners of an event, 10, before it warns.
    #[link_name = "get events#EventEmitter.defaultMaxListeners"]
    pub safe fn default_max_listeners() -> f64;

    #[link_name = "set events#EventEmitter.defaultMaxListeners"]
    pub safe fn set_default_max_listeners(value: f64);
}

/// What [`event_emitter::new_with_options`] takes.
#[derive(Default)]
pub struct EventEmitterOptions {
    /// Whether a listener's rejected promise is its `"error"`.
    #[cfg_attr(rust_js, rust_js::name = "captureRejections")]
    pub capture_rejections: Option<bool>,
}

/// What [`once_with_options`] takes.
#[derive(Default)]
pub struct StaticEventEmitterOptions<'a> {
    /// What aborts the wait.
    pub signal: Option<&'a webapi::AbortSignal>,
}

/// [`Abortable`](https://nodejs.org/api/events.html): an operation's options
/// of a signal that aborts it.
#[derive(Default)]
pub struct Abortable<'a> {
    pub signal: Option<&'a webapi::AbortSignal>,
}
