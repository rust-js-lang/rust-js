//! createDetails' whole shape, of ADR 1164's forms: upstream's
//! kernel/details.ts (vercel-labs/ui@220c1a0) of five of its reasons, which
//! test/create-details.test.ts holds to Codex's contract.
use std::cell::Cell;
use std::rc::Rc;
use webapi::{DragEvent, Event, FocusEvent, KeyboardEvent, MouseEvent};

/// How the user caused a change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeSource {
    #[cfg_attr(rust_js, rust_js::name = "mouse")]
    Mouse,
    #[cfg_attr(rust_js, rust_js::name = "touch")]
    Touch,
    #[cfg_attr(rust_js, rust_js::name = "pen")]
    Pen,
    #[cfg_attr(rust_js, rust_js::name = "keyboard")]
    Keyboard,
    #[cfg_attr(rust_js, rust_js::name = "virtual")]
    Virtual,
    #[cfg_attr(rust_js, rust_js::name = "programmatic")]
    Programmatic,
    #[cfg_attr(rust_js, rust_js::name = "agent")]
    Agent,
}

/// A single-element control was pressed.
#[cfg_attr(rust_js, rust_js::name = "press")]
#[derive(Clone, Copy, Debug)]
pub struct Press;

/// Escape.
#[cfg_attr(rust_js, rust_js::name = "escape-key")]
#[derive(Clone, Copy, Debug)]
pub struct EscapeKey;

/// Enter, or leaving a number input, committed what was typed.
#[cfg_attr(rust_js, rust_js::name = "input-commit")]
#[derive(Clone, Copy, Debug)]
pub struct InputCommit;

/// Files were dropped on a drop zone.
#[cfg_attr(rust_js, rust_js::name = "drop")]
#[derive(Clone, Copy, Debug)]
pub struct Dropped;

/// A form reset.
#[cfg_attr(rust_js, rust_js::name = "reset")]
#[derive(Clone, Copy, Debug)]
pub struct Reset;

/// The DOM event behind a press.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum PressEvent {
    Mouse(&'static MouseEvent),
    Keyboard(&'static KeyboardEvent),
}

/// The DOM event behind a commit.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum CommitEvent {
    Focus(&'static FocusEvent),
    Keyboard(&'static KeyboardEvent),
}

/// Every reason a change can report, and the DOM event behind it.
#[cfg_attr(
    rust_js,
    rust_js::type_map(map = "ReasonVocabulary", keys = "ChangeReason", lookup = "EventOf")
)]
#[cfg_attr(rust_js, rust_js::types = "string")]
pub trait Reason: Copy {
    type Event;
}

impl Reason for Press {
    type Event = PressEvent;
}

impl Reason for EscapeKey {
    type Event = &'static KeyboardEvent;
}

impl Reason for InputCommit {
    type Event = CommitEvent;
}

impl Reason for Dropped {
    type Event = &'static DragEvent;
}

impl Reason for Reset {
    type Event = ();
}

impl Reason for &'static str {
    type Event = &'static Event;
}

#[cfg_attr(rust_js, rust_js::types = "ChangeReason")]
pub trait SharedReason: Reason {}

impl SharedReason for EscapeKey {}

/// What every change callback receives, one member per reason.
#[cfg_attr(rust_js, rust_js::distributive)]
pub struct ChangeDetails<R: Reason = &'static str> {
    /// Why it changed.
    pub reason: R,
    /// The DOM event behind it.
    pub event: Option<R::Event>,
    /// How the user caused it.
    pub source: ChangeSource,
    /// Stops the change.
    pub cancel: Box<dyn Fn()>,
    /// Whether `cancel()` was called.
    pub canceled: js::Accessor<bool>,
}

/// What a commit callback receives, one member per reason.
#[cfg_attr(rust_js, rust_js::distributive)]
pub struct CommitDetails<R: Reason = &'static str> {
    /// What changed the value.
    pub reason: R,
    /// The DOM event that ended the interaction.
    pub event: Option<R::Event>,
    /// How the user caused it.
    pub source: ChangeSource,
}

/// What details are given: a reason, and an event if any.
#[cfg_attr(rust_js, rust_js::types = "{ reason: string; event?: Event }")]
pub trait Given {}

/// A change's or a commit's details, as `cancelable` says, with what's given.
#[cfg_attr(
    rust_js,
    rust_js::types = "(C extends true ? ChangeDetails<G[\"reason\"]> : CommitDetails<G[\"reason\"]>) & Omit<G, \"reason\" | \"event\" | \"source\">"
)]
type Details<G, C> = (&'static G, C);

struct Committed<G: 'static> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    given: js::Spread<&'static G>,
    source: ChangeSource,
}

struct Changed<G: 'static> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    given: js::Spread<&'static G>,
    source: ChangeSource,
    cancel: Box<dyn Fn()>,
    canceled: js::Accessor<bool>,
}

// Each is a JS object, never `undefined`.
unsafe impl<G> js::Defined for Committed<G> {}
unsafe impl<G> js::Defined for Changed<G> {}

/// The details a callback receives: what's `given`, with how the user caused
/// it, and of a change, its `cancel()` and `canceled`.
///
/// # Safety
///
/// `given`'s event must be one its reason reports, which isn't checked, as
/// upstream's cast trusts it.
#[cfg_attr(rust_js, rust_js::none_undefined(cancelable))]
pub unsafe fn create_details<#[cfg_attr(rust_js, rust_js::const_param)] G, C>(
    given: &'static G,
    source: ChangeSource,
    cancelable: Option<C>,
) -> Details<G, C>
where
    G: Given + 'static,
    C: js::Boolean + js::DefaultsTo<js::True>,
{
    let cancelable = cancelable.map(js::boolean).unwrap_or(true);
    if !cancelable {
        return unsafe {
            js::cast(Some(js::unknown_of(Committed {
                given: js::Spread::new(given),
                source,
            })))
        };
    }
    let canceled = Rc::new(Cell::new(false));
    let set = Rc::clone(&canceled);
    unsafe {
        js::cast(Some(js::unknown_of(Changed {
            given: js::Spread::new(given),
            source,
            cancel: Box::new(move || set.set(true)),
            canceled: js::Accessor::new(move || canceled.get()),
        })))
    }
}
