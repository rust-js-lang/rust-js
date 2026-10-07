//! [React's events](https://react.dev/reference/react-dom/components/common#react-event-object),
//! which wrap the DOM's. Each derefs to the one it extends, as React's do:
//! a [`PointerEvent`] is a [`MouseEvent`], which is a [`UIEvent`], which is an [`SyntheticEvent`].
//!
//! Each is of an element, `MouseEvent<T = webapi::Element>`, as React's
//! `MouseEvent<T = Element>` is: a `<button>`'s handler gets a
//! `MouseEvent<webapi::HTMLButtonElement>`, whose `current_target` is the button
//! (ADR 0224). A handler of any element's event is `MouseEvent::widen`ed, and an
//! event `upcast` to any element's.

use core::marker::PhantomData;
use core::ops::Deref;

use js::JsObject;

/// A [React event](https://react.dev/reference/react-dom/components/common#react-event-object):
/// what every handler gets. To TypeScript, @types/react's of the same
/// name, but this one's, `SyntheticEvent`.
#[cfg_attr(rust_js, rust_js::types = "react#SyntheticEvent<T>")]
pub struct SyntheticEvent<T = webapi::Element>(PhantomData<JsObject>, PhantomData<T>);

impl<T> SyntheticEvent<T> {
    /// Stop the browser's default action, like submitting a form.
    #[cfg_attr(rust_js, rust_js::link_name = "preventDefault")]
    pub fn prevent_default(&self) {
        unreachable!()
    }

    /// Stop parents' handlers seeing it.
    #[cfg_attr(rust_js, rust_js::link_name = "stopPropagation")]
    pub fn stop_propagation(&self) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "isDefaultPrevented")]
    pub fn is_default_prevented(&self) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "isPropagationStopped")]
    pub fn is_propagation_stopped(&self) -> bool {
        unreachable!()
    }
}

/// Getters, one per React event field: `client_x` is `e.clientX`.
macro_rules! fields {
    ($type:ident { $($(#[doc = $doc:literal])* $method:ident: $ty:ty = $js:literal;)* }) => {
        impl<T: 'static> $type<T> {
            $(
                $(#[doc = $doc])*
                #[cfg_attr(rust_js, rust_js::link_name = concat!("get ", $js))]
                pub fn $method(&self) -> $ty {
                    unreachable!()
                }
            )*
        }
    };
}

fields!(SyntheticEvent {
    bubbles: bool = "bubbles";
    cancelable: bool = "cancelable";
    /// The element whose handler this is: a `<button>`'s is an `HTMLButtonElement`.
    current_target: &'static T = "currentTarget";
    default_prevented: bool = "defaultPrevented";
    event_phase: u32 = "eventPhase";
    is_trusted: bool = "isTrusted";
    /// Where it happened: the element, or one inside it, or what isn't an
    /// element, as `EventTarget` is in the DOM and in @types/react.
    target: &'static webapi::EventTarget = "target";
    time_stamp: f64 = "timeStamp";
    /// The DOM's event that this wraps.
    native_event: &'static webapi::Event = "nativeEvent";
    /// Its name, like `"click"`.
    type_: String = "type";
});

/// What any element's event is to one of an element (ADR 0224): a handler of
/// any element's, widened, and an element's event, upcast. Each is the value
/// itself in JS.
macro_rules! widen {
    ($name:ident) => {
        impl $name {
            /// A handler of any element's event, where an element's is wanted:
            /// `<button onClick={event::MouseEvent::widen(on_click)}>`. One that takes
            /// any element's event takes a button's.
            #[cfg_attr(rust_js, rust_js::link_name = "this")]
            pub fn widen<T: webapi::IsA<webapi::Element>>(this: Box<dyn Fn(&$name)>) -> Box<dyn Fn(&$name<T>)> {
                unreachable!()
            }
        }

        impl<T: 'static> $name<T> {
            /// An async handler, `event::MouseEvent::spawn(async move |e| { .. })`:
            /// each event runs it, without waiting for it, as `js::spawn` runs a
            /// future. In JS it's the async function itself, `async (e) => { .. }`,
            /// whose promise React ignores, as it does what any handler returns.
            #[cfg_attr(rust_js, rust_js::link_name = "this")]
            #[allow(unused_variables)]
            pub fn spawn(this: impl AsyncFn(&$name<T>) + 'static) -> Box<dyn Fn(&$name<T>)> {
                unreachable!()
            }
        }

        impl<T: webapi::IsA<webapi::Element>> $name<T> {
            /// This event, as any element's: for a handler that takes any
            /// element's, `move |e| f(e.upcast())`.
            #[cfg_attr(rust_js, rust_js::link_name = "this")]
            pub fn upcast(&self) -> &$name {
                unreachable!()
            }
        }
    };
}

widen!(SyntheticEvent);

/// Declares an event type that extends another, and what it is to TypeScript.
macro_rules! events {
    ($($(#[doc = $doc:literal])* $name:ident as $ts:literal: $parent:ident { $($body:tt)* })*) => {
        $(
            $(#[doc = $doc])*
            #[cfg_attr(rust_js, rust_js::types = $ts)]
            pub struct $name<T = webapi::Element>(PhantomData<JsObject>, PhantomData<T>);

            impl<T> Deref for $name<T> {
                type Target = $parent<T>;

                fn deref(&self) -> &$parent<T> {
                    // Never runs: rust-js compiles this `Deref` to the object itself.
                    unsafe { &*(self as *const Self as *const $parent<T>) }
                }
            }

            fields!($name { $($body)* });
            widen!($name);
        )*
    };
}

events! {
    /// A [UI event](https://developer.mozilla.org/docs/Web/API/UIEvent), like a scroll.
    UIEvent as "react#UIEvent<T>": SyntheticEvent {
        detail: i32 = "detail";
        view: &'static webapi::Window = "view";
    }

    /// A click, or another [mouse event](https://developer.mozilla.org/docs/Web/API/MouseEvent).
    MouseEvent as "react#MouseEvent<T>": UIEvent {
        alt_key: bool = "altKey";
        /// Which button: 0 is the main one.
        button: i32 = "button";
        buttons: i32 = "buttons";
        client_x: f64 = "clientX";
        client_y: f64 = "clientY";
        ctrl_key: bool = "ctrlKey";
        meta_key: bool = "metaKey";
        movement_x: f64 = "movementX";
        movement_y: f64 = "movementY";
        page_x: f64 = "pageX";
        page_y: f64 = "pageY";
        related_target: Option<&'static webapi::Element> = "relatedTarget";
        screen_x: f64 = "screenX";
        screen_y: f64 = "screenY";
        shift_key: bool = "shiftKey";
    }

    /// A [pointer event](https://developer.mozilla.org/docs/Web/API/PointerEvent):
    /// mouse, pen or touch.
    PointerEvent as "react#PointerEvent<T>": MouseEvent {
        height: f64 = "height";
        is_primary: bool = "isPrimary";
        pointer_id: i32 = "pointerId";
        /// `"mouse"`, `"pen"` or `"touch"`.
        pointer_type: String = "pointerType";
        pressure: f64 = "pressure";
        tangential_pressure: f64 = "tangentialPressure";
        tilt_x: f64 = "tiltX";
        tilt_y: f64 = "tiltY";
        twist: f64 = "twist";
        width: f64 = "width";
    }

    /// A [drag event](https://developer.mozilla.org/docs/Web/API/DragEvent).
    /// Call `prevent_default` in `on_drag_over` to allow a drop.
    DragEvent as "react#DragEvent<T>": MouseEvent {
        data_transfer: &'static webapi::DataTransfer = "dataTransfer";
    }

    /// A [wheel event](https://developer.mozilla.org/docs/Web/API/WheelEvent).
    WheelEvent as "react#WheelEvent<T>": MouseEvent {
        delta_mode: u32 = "deltaMode";
        delta_x: f64 = "deltaX";
        delta_y: f64 = "deltaY";
        delta_z: f64 = "deltaZ";
    }

    /// FocusEvent coming or going: `on_focus` and `on_blur`, which bubble in React.
    FocusEvent as "react#FocusEvent<T>": UIEvent {
        /// Where focus went, or came from.
        related_target: Option<&'static webapi::Element> = "relatedTarget";
    }

    /// A key pressed or let go.
    KeyboardEvent as "react#KeyboardEvent<T>": UIEvent {
        alt_key: bool = "altKey";
        /// Which key it is on the keyboard, like `"KeyA"`.
        code: String = "code";
        ctrl_key: bool = "ctrlKey";
        /// What the key means, like `"Enter"` or `"a"`.
        key: String = "key";
        locale: String = "locale";
        location: u32 = "location";
        meta_key: bool = "metaKey";
        repeat: bool = "repeat";
        shift_key: bool = "shiftKey";
    }

    /// A [touch event](https://developer.mozilla.org/docs/Web/API/TouchEvent).
    TouchEvent as "react#TouchEvent<T>": UIEvent {
        alt_key: bool = "altKey";
        changed_touches: &'static webapi::TouchList = "changedTouches";
        ctrl_key: bool = "ctrlKey";
        meta_key: bool = "metaKey";
        shift_key: bool = "shiftKey";
        target_touches: &'static webapi::TouchList = "targetTouches";
        touches: &'static webapi::TouchList = "touches";
    }

    /// A CSS [animation event](https://developer.mozilla.org/docs/Web/API/AnimationEvent).
    AnimationEvent as "react#AnimationEvent<T>": SyntheticEvent {
        animation_name: String = "animationName";
        elapsed_time: f64 = "elapsedTime";
        pseudo_element: String = "pseudoElement";
    }

    /// A CSS [transition event](https://developer.mozilla.org/docs/Web/API/TransitionEvent).
    TransitionEvent as "react#TransitionEvent<T>": SyntheticEvent {
        elapsed_time: f64 = "elapsedTime";
        property_name: String = "propertyName";
        pseudo_element: String = "pseudoElement";
    }

    /// Copying, cutting or pasting.
    ClipboardEvent as "react#ClipboardEvent<T>": SyntheticEvent {
        clipboard_data: &'static webapi::DataTransfer = "clipboardData";
    }

    /// Text being composed with an input method.
    CompositionEvent as "react#CompositionEvent<T>": SyntheticEvent {
        data: String = "data";
    }

    /// `on_before_input`: text about to be typed.
    InputEvent as "react#InputEvent<T>": SyntheticEvent {
        data: Option<String> = "data";
    }

    /// A popover or `<details>` opening or closing: `on_toggle`, `on_before_toggle`.
    ToggleEvent as "react#ToggleEvent<T>": SyntheticEvent {
        /// `"open"` or `"closed"`.
        new_state: String = "newState";
        old_state: String = "oldState";
    }

    /// An `<input>`, `<select>` or `<textarea>` changing: `on_change`, `on_input`.
    ChangeEvent as "react#ChangeEvent<T>": SyntheticEvent {
        /// What's in it now: `e.target.value`.
        value: String = "target.value";
        /// Whether a checkbox is checked now: `e.target.checked`.
        checked: bool = "target.checked";
    }
}

/// A handler of an event, as @types/react's `EventHandler<E>`:
/// `on_click: MouseEventHandler<HTMLButtonElement>` is TypeScript's
/// `onClick: MouseEventHandler<HTMLButtonElement>`.
pub type EventHandler<E> = Box<dyn Fn(&E)>;
pub type ReactEventHandler<T = webapi::Element> = EventHandler<SyntheticEvent<T>>;
pub type ClipboardEventHandler<T = webapi::Element> = EventHandler<ClipboardEvent<T>>;
pub type CompositionEventHandler<T = webapi::Element> = EventHandler<CompositionEvent<T>>;
pub type DragEventHandler<T = webapi::Element> = EventHandler<DragEvent<T>>;
pub type FocusEventHandler<T = webapi::Element> = EventHandler<FocusEvent<T>>;
/// Of one element: @types/react's takes the target's too, which this
/// `ChangeEvent`'s `value` reads without.
pub type ChangeEventHandler<T = webapi::Element> = EventHandler<ChangeEvent<T>>;
pub type InputEventHandler<T = webapi::Element> = EventHandler<InputEvent<T>>;
pub type KeyboardEventHandler<T = webapi::Element> = EventHandler<KeyboardEvent<T>>;
pub type MouseEventHandler<T = webapi::Element> = EventHandler<MouseEvent<T>>;
pub type TouchEventHandler<T = webapi::Element> = EventHandler<TouchEvent<T>>;
pub type PointerEventHandler<T = webapi::Element> = EventHandler<PointerEvent<T>>;
pub type UIEventHandler<T = webapi::Element> = EventHandler<UIEvent<T>>;
pub type WheelEventHandler<T = webapi::Element> = EventHandler<WheelEvent<T>>;
pub type AnimationEventHandler<T = webapi::Element> = EventHandler<AnimationEvent<T>>;
pub type ToggleEventHandler<T = webapi::Element> = EventHandler<ToggleEvent<T>>;
pub type TransitionEventHandler<T = webapi::Element> = EventHandler<TransitionEvent<T>>;

impl<T> MouseEvent<T> {
    /// Whether a modifier key, like `"Shift"` or `"CapsLock"`, is down.
    #[cfg_attr(rust_js, rust_js::link_name = "getModifierState")]
    pub fn get_modifier_state(&self, key: &str) -> bool {
        unreachable!()
    }
}

impl<T> KeyboardEvent<T> {
    #[cfg_attr(rust_js, rust_js::link_name = "getModifierState")]
    pub fn get_modifier_state(&self, key: &str) -> bool {
        unreachable!()
    }
}
