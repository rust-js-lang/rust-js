//! [React](https://react.dev) and React DOM for rust-js (ADRs 0041, 0043). A
//! component is a function that returns an [`Element`], and elements are
//! written with `jsx!`, which rust-js prints as the JSX you would write by
//! hand (ADR 0075):
//!
//! ```ignore
//! use react::{Element, use_state};
//!
//! pub fn Counter() -> Element {
//!     let (count, set_count) = use_state(0);
//!     jsx! {
//!         <div className="counter">
//!             <button onClick={move |_| set_count.update(|count| count + 1)}>{"+"}</button>
//!             {count}
//!         </div>
//!     }
//! }
//! ```
//!
//! ```js
//! import { useState } from "react";
//!
//! export function Counter() {
//!   const [count, setCount] = useState(0);
//!   return <div className="counter">
//!     <button onClick={() => setCount((count) => count + 1 | 0)}>+</button>
//!     {count}
//!   </div>;
//! }
//! ```
//!
//! Every item here is a binding: rustc checks the types, and the bodies never
//! run. Generic ones use `#[rust_js::link_name]` (ADR 0039).
//!
//! # React versions
//!
//! This crate binds React's API as of the latest release, from 18.0 on. What
//! a later release added is gated by it: `#[cfg(react = "19.2")]` is "React
//! 19.2 or later". `react/build.sh --react <version>` builds the crate for the
//! version a project has installed, so using what that version lacks is a
//! compile error, not a crash in the browser (ADR 0043). `react/versions.json`
//! records which release first has each export, event and attribute, read
//! from the releases themselves.

// The bodies are never compiled, so they don't use their parameters.
#![allow(unused_variables)]

#[cfg(react = "19.0")]
use core::future::Future;
use core::marker::PhantomData;
use core::ops::Deref;
use std::thread::LocalKey;

use js::{JsError, JsObject, Promise, Unknown};

/// Each element's attributes, as @types/react types them, for props to
/// flatten: `#[rust_js::flatten] anchor: attributes::AnchorHtmlAttributes<'a>`.
pub mod attributes;
pub mod children;
pub mod dom;
mod elements;
pub mod event;

#[doc(hidden)]
pub use elements::html;
/// The DOM, whose types React's APIs use: `react::webapi::FormData`.
pub use js;
pub use webapi;

/// A React element: what a component returns, and what goes in children.
/// Construct it with `jsx! { <Tag ... /> }`.
///
/// While `jsx!` builds a tag's, it's of that tag's DOM element,
/// `Element<webapi::HTMLButtonElement>`, which its handlers' events and its
/// `ref` take, as @types/react's `IntrinsicElements` gives them (ADR 0224);
/// what it makes is an `Element`, whatever its tag.
#[cfg_attr(rust_js, rust_js::jsx_element)]
#[cfg_attr(rust_js, rust_js::types = "react#ReactNode<>")]
pub struct Element<T = webapi::Element>(PhantomData<JsObject>, PhantomData<T>);

/// A tag's element, as what `jsx!` makes: an `Element`, the value itself.
#[doc(hidden)]
#[cfg_attr(rust_js, rust_js::link_name = "this")]
pub fn element<T>(this: Element<T>) -> Element {
    unreachable!()
}

/// The props a component's struct doesn't name, JS's `...rest`: a field of
/// one, in props the component destructures, `LinkProps { href, rest }:
/// LinkProps`, holds what a JS caller gave besides, which `<a {...rest}>`
/// spreads onto an element. Its default is none (ADR 0195).
#[cfg_attr(rust_js, rust_js::rest_props)]
pub struct Rest(PhantomData<JsObject>);

impl Default for Rest {
    fn default() -> Self {
        unreachable!("rust-js writes `undefined`")
    }
}

/// React's empty node, `undefined`, which renders nothing: what a props
/// struct that derives `Default` has of its `children` where they aren't
/// given, as a JS component's binding is, `next/link`'s (ADR 0192).
impl Default for Element {
    fn default() -> Self {
        unreachable!("rust-js writes `undefined`")
    }
}

/// JSX, `jsx! { <h1>{"Hi"}</h1> }`, which rust-js compiles itself (ADR 0040):
/// to rust-js, the Rust it writes for it, which it gives as `@rust_js ..`; to
/// a plain rustc, a user's own `cargo check`, an `Element` it doesn't look
/// inside (ADR 0113). A file with JSX imports it, `use react::jsx;`.
#[macro_export]
macro_rules! jsx {
    (@rust_js $($rust:tt)*) => { $($rust)* };
    ($($jsx:tt)*) => { $crate::Element::__jsx() };
}

/// What React renders as a child: elements, text and numbers, and tuples,
/// `Vec`s and `Option`s of them. A tuple is several children, `("Count is ",
/// count)`; a `Vec` is a list, whose items each need a [`key`](Element::key);
/// `None` is nothing. Each is the JS value React expects already, so nothing
/// converts them. Only these are: what each one's `Default` makes is
/// std's or React's, which rust-js knows does nothing else. To TypeScript,
/// a type parameter of one is a `ReactNode`: `children: C` of a `C: ReactNode`
/// is `children: ReactNode`.
#[cfg_attr(rust_js, rust_js::jsx_node)]
#[cfg_attr(rust_js, rust_js::types = "react#ReactNode<>")]
pub trait ReactNode: sealed::Sealed {}

mod sealed {
    pub trait Sealed {}
}

impl ReactNode for Element {}
impl ReactNode for &str {}
impl ReactNode for String {}
impl ReactNode for () {}
impl ReactNode for bool {}
impl<T: ReactNode + ?Sized> ReactNode for &T {}
impl<T: ReactNode> ReactNode for Option<T> {}
impl<T: ReactNode> ReactNode for Vec<T> {}
impl<T: ReactNode> ReactNode for [T] {}
/// What JS gives untyped, a child's `props.children` say, as JSX renders it.
impl ReactNode for Unknown {}
impl sealed::Sealed for Element {}
impl sealed::Sealed for &str {}
impl sealed::Sealed for String {}
impl sealed::Sealed for () {}
impl sealed::Sealed for bool {}
impl<T: ReactNode + ?Sized> sealed::Sealed for &T {}
impl<T: ReactNode> sealed::Sealed for Option<T> {}
impl<T: ReactNode> sealed::Sealed for Vec<T> {}
impl<T: ReactNode> sealed::Sealed for [T] {}
impl sealed::Sealed for Unknown {}

/// A node, told apart by what it is, as JSX's `typeof children ===
/// "string"`, `isValidElement(children)` and `Array.isArray(children)`:
/// text, an element, a list of nodes, as JSX gives several children, or any
/// other node, of [`kind_of`] (ADR 0214). Each is a node itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ReactNodeKind<'a> {
    Text(&'a str),
    Element(&'a ReactElement),
    List(&'a [ReactNodeKind<'a>]),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(&'a JsObject),
}

impl ReactNode for ReactNodeKind<'_> {}
impl sealed::Sealed for ReactNodeKind<'_> {}

/// What `node` is, its text, an element or another node: the node itself,
/// matched, `if let ReactNodeKind::Text(text) = kind_of(&children)`.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn kind_of<C: ReactNode>(this: &C) -> ReactNodeKind<'_> {
    unreachable!()
}

/// A React element, what JSX makes, `{ type, props, key }`, as
/// [`isValidElement`](https://react.dev/reference/react/isValidElement)
/// tells one: a node that's one, of [`kind_of`] or [`children::to_array`].
#[cfg_attr(rust_js, rust_js::test = "react#isValidElement")]
#[cfg_attr(rust_js, rust_js::types = "react#ReactElement")]
pub struct ReactElement(PhantomData<JsObject>);

impl ReactElement {
    /// Its `type`: a DOM element's tag, `"img"`, or a component, whose own
    /// properties `js::get` reads.
    #[cfg_attr(rust_js, rust_js::link_name = "get type")]
    pub fn r#type(&self) -> &'static Unknown {
        unreachable!()
    }

    /// Its `props`, `children` among them, which `js::get` reads.
    #[cfg_attr(rust_js, rust_js::link_name = "get props")]
    pub fn props(&self) -> &'static Unknown {
        unreachable!()
    }

    /// Its `key`, `None` where it has none.
    #[cfg_attr(rust_js, rust_js::link_name = "get key")]
    pub fn key(&self) -> Option<&'static str> {
        unreachable!()
    }

    /// It, as what JSX makes, a component's result say. The element itself.
    #[cfg_attr(rust_js, rust_js::link_name = "this")]
    pub fn element(&self) -> Element {
        unreachable!()
    }
}

impl ReactNode for ReactElement {}
impl sealed::Sealed for ReactElement {}

/// [`cloneElement`](https://react.dev/reference/react/cloneElement):
/// `element` again, `props`' fields over its own,
/// `clone_element(child, Linked { is_link: true })`: an element, as
/// @types/react's `ReactElement`, so a child as any other, `Child::Element`.
#[cfg_attr(rust_js, rust_js::link_name = "react#cloneElement")]
#[allow(unused_variables)]
pub fn clone_element<P>(element: &ReactElement, props: P) -> &'static ReactElement {
    unreachable!()
}

macro_rules! numbers {
    ($($t:ty),*) => { $(impl ReactNode for $t {} impl sealed::Sealed for $t {} impl Value for $t {})* };
}
numbers!(i8, i16, i32, u8, u16, u32, usize, f64);

macro_rules! tuples {
    ($($name:ident)+) => {
        impl<$($name: ReactNode),+> ReactNode for ($($name,)+) {}
        impl<$($name: ReactNode),+> sealed::Sealed for ($($name,)+) {}
        impl<$($name),+> DependencyList for ($($name,)+) {}
    };
}
tuples!(A);
tuples!(A B);
tuples!(A B C);
tuples!(A B C D);
tuples!(A B C D E);
tuples!(A B C D E F);
tuples!(A B C D E F G);
tuples!(A B C D E F G H);
tuples!(A B C D E F G H I);
tuples!(A B C D E F G H I J);
tuples!(A B C D E F G H I J K);
tuples!(A B C D E F G H I J K L);

/// What an attribute or style takes: text, a number (which React turns into
/// text, or pixels in a style), a `bool`, or an `Option` of one, which leaves
/// it out when `None`.
pub trait Value {}

impl Value for &str {}
impl Value for String {}
impl Value for bool {}
impl<T: Value + ?Sized> Value for &T {}
impl<T: Value> Value for Option<T> {}

/// What a [`style`](Element::style) can be: a [`CSSProperties`], or an `Option` of
/// one, which leaves it out when `None`, as a component's optional `style`
/// prop passed on.
pub trait StyleValue {}

impl StyleValue for CSSProperties {}
impl StyleValue for Option<CSSProperties> {}

/// What a [`key`](Element::key) can be: a string or a number.
pub trait Key {}

impl Key for &str {}
impl Key for String {}
impl Key for i32 {}
impl Key for u32 {}
impl Key for usize {}
impl<T: Key + ?Sized> Key for &T {}

/// A [`style`](Element::style) object, `{ color: "red", fontSize: 12 }`:
/// made by `CSSProperties::new()`, then CSS properties by name, `.color("red")`.
/// To TypeScript, React's `CSSProperties`, as a tag's `style` is.
#[cfg_attr(rust_js, rust_js::types = "react#CSSProperties")]
pub struct CSSProperties(PhantomData<JsObject>);

impl CSSProperties {
    #[cfg_attr(rust_js, rust_js::link_name = "{}")]
    pub fn new() -> CSSProperties {
        unreachable!()
    }

    /// Another style's properties over these, `{ width, ...customStyles }`,
    /// as react.dev's console box spreads its own; `None` spreads nothing.
    #[cfg_attr(rust_js, rust_js::link_name = "prop ...")]
    pub fn spread(self, other: impl StyleValue) -> CSSProperties {
        unreachable!()
    }

    /// Any property, like a custom one: `.set("--accent", "red")`. The name
    /// is a string literal.
    #[cfg_attr(rust_js, rust_js::link_name = "prop")]
    pub fn set(self, name: &'static str, value: impl Value) -> CSSProperties {
        unreachable!()
    }
}

/// `{ __html }`, for [`dangerously_set_inner_html`](Element::dangerously_set_inner_html).
pub struct InnerHtml(PhantomData<JsObject>);

#[cfg_attr(rust_js, rust_js::link_name = "{__html}")]
pub fn inner_html(html: impl Value) -> InnerHtml {
    unreachable!()
}

#[doc(hidden)]
impl Element {
    /// What `jsx!` is to a plain rustc, which doesn't compile JSX: named so
    /// no prop is, as a prop is a method.
    #[doc(hidden)]
    pub fn __jsx() -> Element {
        unreachable!()
    }
}

#[doc(hidden)]
impl<T> Element<T> {
    /// Spread a props struct into this element. Fields keep the JS names of
    /// their Rust struct (including the crate's camel_case setting).
    #[cfg_attr(rust_js, rust_js::link_name = "prop ...")]
    pub fn props<P>(self, props: P) -> Element<T> {
        unreachable!()
    }

    /// Its children: one [`ReactNode`], or several as a tuple.
    #[cfg_attr(rust_js, rust_js::link_name = "prop children")]
    pub fn children(self, children: impl ReactNode) -> Element<T> {
        unreachable!()
    }

    /// Tells React which item of a list this is, across renders.
    #[cfg_attr(rust_js, rust_js::link_name = "prop key")]
    pub fn key(self, key: impl Key) -> Element<T> {
        unreachable!()
    }

    /// A ref to its element, or to one its element extends: a `<button>`'s
    /// holds an `HTMLButtonElement`, or an `Element` (ADR 0224).
    #[cfg_attr(rust_js, rust_js::link_name = "prop ref")]
    pub fn r#ref<U: 'static, M>(self, value: impl RefValue<&'static U, M>) -> Element<T>
    where
        T: webapi::IsA<U>,
    {
        unreachable!()
    }

    /// Any attribute, by its name in JSX, which must be a string literal:
    /// `.attr("aria-hidden", "true")`, `.attr("data-id", id)`.
    #[cfg_attr(rust_js, rust_js::link_name = "prop")]
    pub fn attr(self, name: &'static str, value: impl Value) -> Element<T> {
        unreachable!()
    }

    /// [`style`](https://react.dev/reference/react-dom/components/common#applying-css-styles):
    /// `.style(CSSProperties::new().color("red"))` is `style={{ color: "red" }}`,
    /// and of an `Option<CSSProperties>`, none where it's `None`.
    #[cfg_attr(rust_js, rust_js::link_name = "prop style")]
    pub fn style(self, style: impl StyleValue) -> Element<T> {
        unreachable!()
    }

    /// `dangerouslySetInnerHTML={{ __html }}`: HTML that React doesn't escape.
    #[cfg_attr(rust_js, rust_js::link_name = "prop dangerouslySetInnerHTML")]
    pub fn dangerously_set_inner_html(self, html: InnerHtml) -> Element<T> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "prop action")]
    pub fn action<M>(self, value: impl FormAction<M>) -> Element<T> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "prop formAction")]
    pub fn form_action<M>(self, value: impl FormAction<M>) -> Element<T> {
        unreachable!()
    }

    /// A stylesheet's or style's place among others, which React orders and
    /// hoists into `<head>`.
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "prop precedence")]
    pub fn precedence(self, value: impl Value) -> Element<T> {
        unreachable!()
    }
}

/// What an action returns: nothing, or, from React 19, a future, which
/// React awaits while its Transition is pending. `M` only tells them apart.
pub trait ActionResult<M> {}

pub struct SyncAction;
pub struct AsyncAction;

impl ActionResult<SyncAction> for () {}
#[cfg(react = "19.0")]
impl<F: Future<Output = ()>> ActionResult<AsyncAction> for F {}

/// A JSX ref is a ref object or a callback. React calls a callback on attach
/// and detach; from React 19, its returned cleanup runs on detach.
/// The marker only distinguishes types.
pub trait RefValue<H, M> {}
#[doc(hidden)]
pub struct ObjectRef;
#[doc(hidden)]
pub struct CallbackRef<C>(PhantomData<C>);
impl<H> RefValue<H, ObjectRef> for RefObject<Option<H>> {}
impl<H: 'static, C: Cleanup, F: Fn(Option<H>) -> C + 'static> RefValue<H, CallbackRef<C>> for F {}

/// A JSX form action is a URL or, on React 19+, a function or action dispatch.
pub trait FormAction<M> {}
#[doc(hidden)]
pub struct UrlAction;
#[doc(hidden)]
pub struct FunctionAction<M>(PhantomData<M>);
#[doc(hidden)]
pub struct DispatchAction;
impl<T: Value> FormAction<UrlAction> for T {}
#[cfg(react = "19.0")]
impl<M, R: ActionResult<M>, F: Fn(&'static webapi::FormData) -> R + 'static> FormAction<FunctionAction<M>> for F {}
#[cfg(react = "19.0")]
impl FormAction<DispatchAction> for Dispatch<&'static webapi::FormData> {}

// ── Hooks ───────────────────────────────────────────────────────────────
//
// What a hook gives back is borrowed, `&'static T`: React keeps it, and it's
// read-only, as React's state is (a new value is what renders again). A
// shared reference is `Copy`, so every handler can `move` it in. In JS it's
// the value itself. They're at the crate's root, not re-exported from a
// module, so using one a React version lacks says which version it needs.

/// Declares a JS function a hook gives back, `setCount` or `dispatch`: a
/// `Copy` handle, called with `this()` (ADR 0039).
macro_rules! handle {
    ($(#[doc = $doc:literal])* $name:ident<$t:ident>) => {
        $(#[doc = $doc])*
        pub struct $name<$t>(PhantomData<JsObject>, PhantomData<$t>);

        impl<$t> Clone for $name<$t> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<$t> Copy for $name<$t> {}
    };
}

// ── State ───────────────────────────────────────────────────────────────

/// [`useState`](https://react.dev/reference/react/useState): the state, and
/// what sets it. `let (count, set_count) = use_state(0);` is
/// `const [count, setCount] = useState(0);`.
#[cfg_attr(rust_js, rust_js::link_name = "react#useState")]
pub fn use_state<T>(initial: T) -> (&'static T, Dispatch<SetStateAction<T>>) {
    unreachable!()
}

/// `useState(() => initial())`: the first value, computed on the first render only.
#[cfg_attr(rust_js, rust_js::link_name = "react#useState")]
pub fn use_state_with<T>(initial: impl Fn() -> T + 'static) -> (&'static T, Dispatch<SetStateAction<T>>) {
    unreachable!()
}

/// What [`use_state`]'s setter takes, as @types/react's `SetStateAction<S>`:
/// a value, `set`, or a function of the previous one, `update`. Its setter
/// is a `Dispatch<SetStateAction<T>>`, React's `setCount`.
pub struct SetStateAction<T>(PhantomData<T>);

impl<T> Dispatch<SetStateAction<T>> {
    /// `setCount(value)`.
    #[cfg_attr(rust_js, rust_js::link_name = "this()")]
    pub fn set(&self, value: T) {
        unreachable!()
    }

    /// `setCount((count) => count + 1)`: a new state from the latest one,
    /// which it only reads.
    #[cfg_attr(rust_js, rust_js::link_name = "this()")]
    pub fn update(&self, f: impl Fn(&T) -> T + 'static) {
        unreachable!()
    }
}

/// [`useReducer`](https://react.dev/reference/react/useReducer): the state,
/// and what sends it actions, which `reducer` turns into the next state.
#[cfg_attr(rust_js, rust_js::link_name = "react#useReducer")]
pub fn use_reducer<S, A>(reducer: impl Fn(&S, A) -> S + 'static, initial: S) -> (&'static S, Dispatch<A>) {
    unreachable!()
}

/// `useReducer(reducer, arg, init)`: the first state is `init(arg)`, computed
/// on the first render only.
#[cfg_attr(rust_js, rust_js::link_name = "react#useReducer")]
pub fn use_reducer_with<S, A, I>(
    reducer: impl Fn(&S, A) -> S + 'static,
    arg: I,
    init: impl Fn(I) -> S + 'static,
) -> (&'static S, Dispatch<A>) {
    unreachable!()
}

handle! {
    /// What sends an action: React's `dispatch`, from [`use_reducer`] and
    /// [`use_action_state`].
    Dispatch<A>
}

impl<A> Dispatch<A> {
    /// `dispatch(action)`.
    #[cfg_attr(rust_js, rust_js::link_name = "this()")]
    pub fn dispatch(&self, action: A) {
        unreachable!()
    }
}

/// [`useActionState`](https://react.dev/reference/react/useActionState): the
/// state, an action that runs `action` with the previous state and its
/// payload, and whether one is pending. Give the action to a form with
/// [`Element::action_dispatch`], or call it in a Transition.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react#useActionState")]
pub fn use_action_state<S, P>(
    action: impl Fn(&S, P) -> S + 'static,
    initial: S,
) -> (&'static S, Dispatch<P>, bool) {
    unreachable!()
}

/// [`use_action_state`] with an `async` action: the state is what its
/// future gives, and it's pending until then.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react#useActionState")]
pub fn use_async_action_state<S, P, F: Future<Output = S>>(
    action: impl Fn(&S, P) -> F + 'static,
    initial: S,
) -> (&'static S, Dispatch<P>, bool) {
    unreachable!()
}

/// [`useOptimistic`](https://react.dev/reference/react/useOptimistic):
/// `value`, or what [`SetOptimistic`] set while an action is pending.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react#useOptimistic")]
pub fn use_optimistic<S>(value: S) -> (&'static S, SetOptimistic<S>) {
    unreachable!()
}

/// `useOptimistic(value, reducer)`: the optimistic state is `reducer`'s,
/// from the actions sent to it.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react#useOptimistic")]
pub fn use_optimistic_with<S, A>(value: S, reducer: impl Fn(&S, A) -> S + 'static) -> (&'static S, Dispatch<A>) {
    unreachable!()
}

handle! {
    /// What [`use_optimistic`] gives to set the optimistic state.
    SetOptimistic<S>
}

#[cfg(react = "19.0")]
impl<S> SetOptimistic<S> {
    #[cfg_attr(rust_js, rust_js::link_name = "this()")]
    pub fn set(&self, value: S) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "this()")]
    pub fn update(&self, f: impl Fn(&S) -> S + 'static) {
        unreachable!()
    }
}

// ── Context and refs ────────────────────────────────────────────────────

/// [`useContext`](https://react.dev/reference/react/useContext): the value of
/// the nearest provider above, or the context's default.
#[cfg_attr(rust_js, rust_js::link_name = "react#useContext")]
pub fn use_context<T>(context: &'static LocalKey<Context<T>>) -> &'static T {
    unreachable!()
}

/// [`useRef`](https://react.dev/reference/react/useRef): a box that keeps its
/// value across renders, without rendering again when it changes. For a DOM
/// element, start it with `None` and give it to [`Element::ref`].
#[cfg_attr(rust_js, rust_js::link_name = "react#useRef")]
pub fn use_ref<T>(initial: T) -> RefObject<T> {
    unreachable!()
}

handle! {
    /// What [`use_ref`] gives: `{ current }`.
    RefObject<T>
}

impl<T> RefObject<T> {
    /// `ref.current`.
    #[cfg_attr(rust_js, rust_js::link_name = "get current")]
    pub fn current(&self) -> T {
        unreachable!()
    }

    /// `ref.current = value`.
    #[cfg_attr(rust_js, rust_js::link_name = "set current")]
    pub fn set_current(&self, value: T) {
        unreachable!()
    }
}

/// [`useImperativeHandle`](https://react.dev/reference/react/useImperativeHandle):
/// what a parent's ref to this component gets, made by `create`.
#[cfg_attr(rust_js, rust_js::link_name = "react#useImperativeHandle")]
pub fn use_imperative_handle<H>(r: RefObject<Option<H>>, create: impl Fn() -> H + 'static, deps: impl DependencyList) {
    unreachable!()
}

// ── Effects ─────────────────────────────────────────────────────────────

/// What an effect's dependencies are: a tuple of values, `(count, name)` for
/// `[count, name]`, or `()` for `[]`, when it runs once.
pub trait DependencyList {}

impl DependencyList for () {}
impl<T, const N: usize> DependencyList for [T; N] {}

/// What an effect returns: nothing, or a function that cleans it up.
pub trait Cleanup {}

impl Cleanup for () {}
impl<F: FnOnce() + 'static> Cleanup for F {}

/// A cleanup an effect gives only sometimes: `undefined`, `None`, where it has
/// none, as React takes it.
impl<F: FnOnce() + 'static> Cleanup for Option<F> {}

/// [`useEffect`](https://react.dev/reference/react/useEffect): run `effect`
/// after a render in which `deps` changed.
#[cfg_attr(rust_js, rust_js::link_name = "react#useEffect")]
pub fn use_effect<C: Cleanup>(effect: impl Fn() -> C + 'static, deps: impl DependencyList) {
    unreachable!()
}

/// `useEffect(effect)`: after every render.
#[cfg_attr(rust_js, rust_js::link_name = "react#useEffect")]
pub fn use_effect_on_every_render<C: Cleanup>(effect: impl Fn() -> C + 'static) {
    unreachable!()
}

/// [`useLayoutEffect`](https://react.dev/reference/react/useLayoutEffect):
/// [`use_effect`], before the browser paints.
#[cfg_attr(rust_js, rust_js::link_name = "react#useLayoutEffect")]
pub fn use_layout_effect<C: Cleanup>(effect: impl Fn() -> C + 'static, deps: impl DependencyList) {
    unreachable!()
}

/// `useLayoutEffect(effect)`: after every render, before the browser paints.
#[cfg_attr(rust_js, rust_js::link_name = "react#useLayoutEffect")]
pub fn use_layout_effect_on_every_render<C: Cleanup>(effect: impl Fn() -> C + 'static) {
    unreachable!()
}

/// [`useInsertionEffect`](https://react.dev/reference/react/useInsertionEffect):
/// before layout effects, for CSS-in-JS libraries to insert styles.
#[cfg_attr(rust_js, rust_js::link_name = "react#useInsertionEffect")]
pub fn use_insertion_effect<C: Cleanup>(effect: impl Fn() -> C + 'static, deps: impl DependencyList) {
    unreachable!()
}

/// `useInsertionEffect(effect)`: after every render.
#[cfg_attr(rust_js, rust_js::link_name = "react#useInsertionEffect")]
pub fn use_insertion_effect_on_every_render<C: Cleanup>(effect: impl Fn() -> C + 'static) {
    unreachable!()
}

/// [`useEffectEvent`](https://react.dev/reference/react/useEffectEvent): `f`,
/// which an effect can call and always see the latest props and state, without
/// being one of its dependencies. Call it only from effects.
#[cfg(react = "19.2")]
#[cfg_attr(rust_js, rust_js::link_name = "react#useEffectEvent")]
pub fn use_effect_event<F: 'static>(f: F) -> &'static F {
    unreachable!()
}

// ── Performance ─────────────────────────────────────────────────────────

/// [`useMemo`](https://react.dev/reference/react/useMemo): `f`'s value, computed
/// again only when `deps` change.
#[cfg_attr(rust_js, rust_js::link_name = "react#useMemo")]
pub fn use_memo<T>(f: impl Fn() -> T + 'static, deps: impl DependencyList) -> &'static T {
    unreachable!()
}

/// [`useCallback`](https://react.dev/reference/react/useCallback): the same
/// function across renders, until `deps` change.
#[cfg_attr(rust_js, rust_js::link_name = "react#useCallback")]
pub fn use_callback<F: 'static>(f: F, deps: impl DependencyList) -> &'static F {
    unreachable!()
}

/// [`useTransition`](https://react.dev/reference/react/useTransition): whether
/// a Transition is pending, and what starts one.
#[cfg_attr(rust_js, rust_js::link_name = "react#useTransition")]
pub fn use_transition() -> (bool, TransitionStartFunction) {
    unreachable!()
}

/// What [`use_transition`] gives: React's `startTransition`.
pub struct TransitionStartFunction(PhantomData<JsObject>);

impl Clone for TransitionStartFunction {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for TransitionStartFunction {}

impl TransitionStartFunction {
    /// `startTransition(action)`: the updates in `action` render without
    /// blocking the page. From React 19, `action` can be `async`.
    #[cfg_attr(rust_js, rust_js::link_name = "this()")]
    pub fn start<R: ActionResult<M>, M>(&self, action: impl FnOnce() -> R + 'static) {
        unreachable!()
    }
}

/// [`useDeferredValue`](https://react.dev/reference/react/useDeferredValue):
/// `value`, which lags behind while more urgent updates render.
#[cfg_attr(rust_js, rust_js::link_name = "react#useDeferredValue")]
pub fn use_deferred_value<T>(value: T) -> &'static T {
    unreachable!()
}

/// `useDeferredValue(value, initialValue)`: `initial` on the first render.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react#useDeferredValue")]
pub fn use_deferred_value_with<T>(value: T, initial: T) -> &'static T {
    unreachable!()
}

// ── Other ───────────────────────────────────────────────────────────────

/// [`useId`](https://react.dev/reference/react/useId): an id unique to this
/// component, the same on every render, for `id` and `html_for`.
#[cfg_attr(rust_js, rust_js::link_name = "react#useId")]
pub fn use_id() -> String {
    unreachable!()
}

/// What [`use_sync_external_store`]'s `subscribe` is given: call it when the
/// store changes.
pub struct Notify(PhantomData<JsObject>);

impl Clone for Notify {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Notify {}

impl Notify {
    #[cfg_attr(rust_js, rust_js::link_name = "this()")]
    pub fn call(&self) {
        unreachable!()
    }
}

/// [`useSyncExternalStore`](https://react.dev/reference/react/useSyncExternalStore):
/// a value from outside React. `subscribe` gets a [`Notify`] to call on each
/// change, and returns what unsubscribes; `snapshot` reads the value.
#[cfg_attr(rust_js, rust_js::link_name = "react#useSyncExternalStore")]
pub fn use_sync_external_store<T, U: Cleanup>(
    subscribe: impl Fn(Notify) -> U + 'static,
    snapshot: impl Fn() -> T + 'static,
) -> &'static T {
    unreachable!()
}

/// `useSyncExternalStore(subscribe, snapshot, serverSnapshot)`: on the server
/// and while hydrating, the value is `server_snapshot`'s.
#[cfg_attr(rust_js, rust_js::link_name = "react#useSyncExternalStore")]
pub fn use_sync_external_store_with_server<T, U: Cleanup>(
    subscribe: impl Fn(Notify) -> U + 'static,
    snapshot: impl Fn() -> T + 'static,
    server_snapshot: impl Fn() -> T + 'static,
) -> &'static T {
    unreachable!()
}

/// [`useDebugValue`](https://react.dev/reference/react/useDebugValue): a label
/// for a custom hook in React DevTools.
#[cfg_attr(rust_js, rust_js::link_name = "react#useDebugValue")]
pub fn use_debug_value<T>(value: T) {
    unreachable!()
}

/// `useDebugValue(value, format)`: formatted only when DevTools shows it.
#[cfg_attr(rust_js, rust_js::link_name = "react#useDebugValue")]
pub fn use_debug_value_with<T>(value: T, format: impl Fn(&T) -> String + 'static) {
    unreachable!()
}

// ── Components ──────────────────────────────────────────────────────────

/// An element of a component: `component(Counter, CounterProps { initial: 1 })`
/// is `<Counter initial={1} />`. Its props are a struct, whose `children`
/// What `jsx!` gives a component's prop it isn't given, an `Option` or one
/// with a default: none, `undefined`, which JSX leaves out, and which the
/// component's own default then is (ADR 0213).
#[cfg_attr(rust_js, rust_js::omitted)]
#[doc(hidden)]
pub fn __omitted<T>() -> T {
    unreachable!()
}

/// field, if it has one, is the element's children; `()` for a component that
/// takes none, `component(App, ())`. A component's name starts with an
/// uppercase letter, as JSX and Fast Refresh need.
#[cfg_attr(rust_js, rust_js::link_name = "<*>")]
#[doc(hidden)]
pub fn component<P, M>(component: impl ComponentType<P, M>, props: P) -> Element {
    unreachable!()
}

/// A JSX component: a function from its props to an [`Element`], or
/// one with no props, or one made by [`memo`], [`lazy`] or [`forward_ref`].
/// `M` only tells them apart.
pub trait ComponentType<P, M> {}

pub struct NoProps;
pub struct WithProps;

impl<F: Fn() -> Element> ComponentType<(), NoProps> for F {}
impl<P, F: Fn(P) -> Element> ComponentType<P, WithProps> for F {}

/// A DOM element's tag as a value, as JSX's `<Comp>` of `const Comp =
/// "h1"`: a fieldless enum whose variants are named as tags are, `"h1"`,
/// `"div"`. `jsx!` reads a tag that names a capitalized parameter or `let`
/// of its function, `<Comp>`, as an element of it, which takes what a DOM
/// element takes (ADR 0220).
pub trait Tag: Copy {}

pub struct Intrinsic;

/// A tag given its props whole, `<Comp {...rest} />`, as a component is.
impl<P, T: Tag> ComponentType<P, Intrinsic> for T {}
impl<T: Tag> Tag for &T {}

/// `<Comp>` of a [`Tag`] `Comp`.
#[cfg_attr(rust_js, rust_js::link_name = "<$>")]
#[doc(hidden)]
pub fn tag(tag: impl Tag) -> Element {
    unreachable!()
}

/// Children with nothing around them: `<>..</>`.
#[cfg_attr(rust_js, rust_js::link_name = "<>")]
#[doc(hidden)]
pub fn fragment(children: impl ReactNode) -> Element {
    unreachable!()
}

/// [`<StrictMode>`](https://react.dev/reference/react/StrictMode).
#[cfg_attr(rust_js, rust_js::link_name = "<react#StrictMode>")]
#[doc(hidden)]
pub fn strict_mode(children: impl ReactNode) -> Element {
    unreachable!()
}

/// Declares a built-in component: `name()` makes its element, whose props
/// are set by its methods, and `.children(..)` finishes it.
macro_rules! built_in {
    ($(#[doc = $doc:literal])* $(#[cfg($cfg:meta)])? $name:ident = $make:ident $tag:literal { $($(#[doc = $pdoc:literal])* $prop:ident: $ty:ty = $js:literal;)* }) => {
        $(#[doc = $doc])*
        $(#[cfg($cfg)])?
        #[cfg_attr(rust_js, rust_js::jsx_element)]
        #[doc(hidden)]
        pub struct $name(PhantomData<JsObject>);

        #[doc = concat!("`<", $tag, ">`: set its props, then its children.")]
        $(#[cfg($cfg)])?
        #[cfg_attr(rust_js, rust_js::link_name = concat!("<react#", $tag, ">"))]
        #[doc(hidden)]
        pub fn $make() -> $name {
            unreachable!()
        }

        $(#[cfg($cfg)])?
        impl ReactNode for $name {}

        $(#[cfg($cfg)])?
        impl crate::sealed::Sealed for $name {}

        $(#[cfg($cfg)])?
        #[doc(hidden)]
        impl $name {
            #[cfg_attr(rust_js, rust_js::link_name = "prop key")]
            pub fn key(self, value: impl Key) -> $name {
                unreachable!()
            }

            #[cfg_attr(rust_js, rust_js::link_name = "prop ...")]
            pub fn props<P>(self, value: P) -> $name {
                unreachable!()
            }

            $(
                $(#[doc = $pdoc])*
                #[cfg_attr(rust_js, rust_js::link_name = concat!("prop ", $js))]
                pub fn $prop(self, value: $ty) -> $name {
                    unreachable!()
                }
            )*

            /// Its children, which finish the element.
            #[cfg_attr(rust_js, rust_js::link_name = "prop children")]
            pub fn children(self, children: impl ReactNode) -> Element {
                unreachable!()
            }
        }
    };
}

built_in! {
    /// [`<Fragment>`](https://react.dev/reference/react/Fragment) with a key,
    /// for a list item of several elements. Without one, [`fragment`] is `<>`.
    Fragment = keyed_fragment "Fragment" {}
}

built_in! {
    /// [`<Suspense>`](https://react.dev/reference/react/Suspense): shows
    /// `fallback` until its children stop suspending.
    Suspense = suspense "Suspense" {
        /// What to show while the children load.
        fallback: impl ReactNode = "fallback";
    }
}

built_in! {
    /// [`<Profiler>`](https://react.dev/reference/react/Profiler): measures
    /// how long its children take to render, in development and profiling builds.
    Profiler = profiler "Profiler" {
        id: impl Value = "id";
        /// Called after each commit: `(id, phase, actual_duration,
        /// base_duration, start_time, commit_time)`, the times in milliseconds.
        on_render: impl Fn(&str, Phase, f64, f64, f64, f64) + 'static = "onRender";
    }
}

/// A [`Profiler`]'s render: the first, a later one, or one its effects caused.
pub enum Phase {
    #[cfg_attr(rust_js, rust_js::name = "mount")]
    Mount,
    #[cfg_attr(rust_js, rust_js::name = "update")]
    Update,
    #[cfg_attr(rust_js, rust_js::name = "nested-update")]
    NestedUpdate,
}

built_in! {
    /// [`<Activity>`](https://react.dev/reference/react/Activity): hides its
    /// children and keeps their state, or shows them.
    #[cfg(react = "19.2")]
    Activity = activity "Activity" {
        mode: ActivityMode = "mode";
    }
}

/// Whether an [`Activity`]'s children show.
#[cfg(react = "19.2")]
pub enum ActivityMode {
    #[cfg_attr(rust_js, rust_js::name = "visible")]
    Visible,
    #[cfg_attr(rust_js, rust_js::name = "hidden")]
    Hidden,
}

built_in! {
    /// [`<ViewTransition>`](https://react.dev/reference/react/ViewTransition):
    /// animates its children with the browser's View Transitions when they
    /// change in a Transition. A class prop is `"auto"`, `"none"`, or a CSS
    /// class for the `::view-transition-*` pseudo-elements.
    #[cfg(react = "19.3")]
    ViewTransition = view_transition "ViewTransition" {
        /// Its name, for a shared-element transition between two of them.
        name: impl Value = "name";
        enter: impl Value = "enter";
        exit: impl Value = "exit";
        update: impl Value = "update";
        share: impl Value = "share";
        default: impl Value = "default";
        on_enter: impl Fn(&ViewTransitionInstance, &Vec<String>) + 'static = "onEnter";
        on_exit: impl Fn(&ViewTransitionInstance, &Vec<String>) + 'static = "onExit";
        on_share: impl Fn(&ViewTransitionInstance, &Vec<String>) + 'static = "onShare";
        on_update: impl Fn(&ViewTransitionInstance, &Vec<String>) + 'static = "onUpdate";
    }
}

/// What a [`ViewTransition`]'s event gets: its pseudo-elements, to animate.
#[cfg(react = "19.3")]
pub struct ViewTransitionInstance(PhantomData<JsObject>);

#[cfg(react = "19.3")]
impl ViewTransitionInstance {
    #[cfg_attr(rust_js, rust_js::link_name = "get name")]
    pub fn name(&self) -> String {
        unreachable!()
    }

    /// `::view-transition-old`, `new`, `group` and `image-pair`.
    #[cfg_attr(rust_js, rust_js::link_name = "get old")]
    pub fn old(&self) -> &'static webapi::Element {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get new")]
    pub fn new(&self) -> &'static webapi::Element {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get group")]
    pub fn group(&self) -> &'static webapi::Element {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get imagePair")]
    pub fn image_pair(&self) -> &'static webapi::Element {
        unreachable!()
    }
}

// ── Context, memo, lazy ─────────────────────────────────────────────────
//
// JS makes a context, a memoized or lazy component once, at a module's top
// level. Rust keeps such a value in a `thread_local!`, which rust-js
// compiles to just that, a `const` of its module (ADR 0037):
//
//     thread_local! {
//         static THEME: Context<&'static str> = create_context("light");
//         static FAST_CARD: MemoExoticComponent<CardProps> = memo(Card);
//     }
//
//     const THEME = createContext("light");
//     const FAST_CARD = memo(Card);
//
// Hooks take the key: `use_context(&THEME)`. JSX uses `<FAST_CARD ... />`.

/// A [context](https://react.dev/reference/react/createContext): a value that
/// a component's descendants read with [`use_context`], from the nearest
/// provider above them, or `default` without one.
#[cfg_attr(rust_js, rust_js::types = "react#Context")]
pub struct Context<T>(PhantomData<JsObject>, PhantomData<T>);

/// [`createContext`](https://react.dev/reference/react/createContext), in a `thread_local!`.
#[cfg_attr(rust_js, rust_js::link_name = "react#createContext")]
pub fn create_context<T>(default: T) -> Context<T> {
    unreachable!()
}

/// Props of `<THEME value={value}>{children}</THEME>` on React 19+,
/// or `<THEME.Provider value={value}>{children}</THEME.Provider>` on React 18+.
/// Its children are any node, as a component's are.
pub struct ProviderProps<T, C> {
    pub value: T,
    pub children: C,
}

pub struct ProvidesContext;

#[cfg(react = "19.0")]
impl<T, C: ReactNode> ComponentType<ProviderProps<T, C>, ProvidesContext> for &'static LocalKey<Context<T>> {}

/// `THEME.Provider`, a context's provider in every React version:
/// Used by `<THEME.Provider value={value}>{children}</THEME.Provider>`.
#[cfg_attr(rust_js, rust_js::link_name = "get Provider")]
#[doc(hidden)]
pub fn provider<T>(this: &'static LocalKey<Context<T>>) -> Provider<T> {
    unreachable!()
}

pub struct Provider<T>(PhantomData<JsObject>, PhantomData<T>);

impl<T, C: ReactNode> ComponentType<ProviderProps<T, C>, ProvidesContext> for Provider<T> {}

/// A component that [`memo`] made: React skips rendering it again while its
/// props are the same as last time.
#[cfg_attr(rust_js, rust_js::types = "react#NamedExoticComponent")]
pub struct MemoExoticComponent<P>(PhantomData<JsObject>, PhantomData<P>);

/// [`memo`](https://react.dev/reference/react/memo), in a `thread_local!`.
/// Props are the same when each field is (`Object.is`).
#[cfg_attr(rust_js, rust_js::link_name = "react#memo")]
pub fn memo<P, M>(component: impl ComponentType<P, M>) -> MemoExoticComponent<P> {
    unreachable!()
}

/// `memo(component, arePropsEqual)`: the props are the same when `are_equal` says so.
#[cfg_attr(rust_js, rust_js::link_name = "react#memo")]
pub fn memo_with<P, M>(component: impl ComponentType<P, M>, are_equal: impl Fn(&P, &P) -> bool + 'static) -> MemoExoticComponent<P> {
    unreachable!()
}

pub struct Memoized;

impl<P> ComponentType<P, Memoized> for &'static LocalKey<MemoExoticComponent<P>> {}

/// A component [`lazy`] loads the first time it renders.
pub struct LazyExoticComponent<P>(PhantomData<JsObject>, PhantomData<P>);

/// A JS module whose default export is a component taking `P`, as
/// [`import_module`] loads it.
pub struct Module<P>(PhantomData<JsObject>, PhantomData<P>);

/// [`lazy`](https://react.dev/reference/react/lazy), in a `thread_local!`:
/// `lazy(|| import_module("./Chart.jsx"))`. It suspends while it loads, so
/// render it inside `<Suspense fallback={...}>...</Suspense>`.
#[cfg_attr(rust_js, rust_js::link_name = "react#lazy")]
pub fn lazy<P>(load: impl Fn() -> Promise<Module<P>> + 'static) -> LazyExoticComponent<P> {
    unreachable!()
}

/// `import(specifier)`: a module, loaded when it's first needed. The
/// specifier names the JS file, as the bundler sees it.
#[cfg_attr(rust_js, rust_js::link_name = "import")]
pub fn import_module<P>(specifier: &'static str) -> Promise<Module<P>> {
    unreachable!()
}

pub struct Loaded;

impl<P> ComponentType<P, Loaded> for &'static LocalKey<LazyExoticComponent<P>> {}

/// A component that [`forward_ref`] made: its parent's `ref` reaches it.
pub struct ForwardRefExoticComponent<P, H>(PhantomData<JsObject>, PhantomData<(P, H)>);

/// [`forwardRef`](https://react.dev/reference/react/forwardRef), in a
/// `thread_local!`: `render` gets the props and the parent's ref. From React
/// 19 a component can take `ref` as a prop instead.
#[cfg_attr(rust_js, rust_js::link_name = "react#forwardRef")]
pub fn forward_ref<P, H>(render: impl Fn(P, RefObject<Option<H>>) -> Element + 'static) -> ForwardRefExoticComponent<P, H> {
    unreachable!()
}

pub struct Forwarded;

impl<P, H> ComponentType<P, Forwarded> for &'static LocalKey<ForwardRefExoticComponent<P, H>> {}

/// Check the handle type of a forwarded JSX ref without emitting a runtime call.
#[doc(hidden)]
#[cfg_attr(rust_js, rust_js::link_name = "this")]
pub fn checked_ref<H, M, R: RefValue<H, M>>(this: R) -> R {
    unreachable!()
}

// ── APIs ────────────────────────────────────────────────────────────────

/// [`startTransition`](https://react.dev/reference/react/startTransition):
/// the state updates in `action` render without blocking the page. From
/// React 19, `action` can be `async`.
#[cfg_attr(rust_js, rust_js::link_name = "react#startTransition")]
pub fn start_transition<R: ActionResult<M>, M>(action: impl FnOnce() -> R + 'static) {
    unreachable!()
}

/// [`addTransitionType`](https://react.dev/reference/react/addTransitionType):
/// names what the current Transition is, for [`ViewTransition`]'s classes.
#[cfg(react = "19.3")]
#[cfg_attr(rust_js, rust_js::link_name = "react#addTransitionType")]
pub fn add_transition_type(name: &str) {
    unreachable!()
}

/// [`act`](https://react.dev/reference/react/act): in a test, apply every
/// update `f` causes before it returns. Development builds only.
#[cfg(react = "18.3")]
#[cfg_attr(rust_js, rust_js::link_name = "react#act")]
pub fn act<R: ActionResult<M>, M>(f: impl FnOnce() -> R + 'static) -> Promise<()> {
    unreachable!()
}

/// [`cache`](https://react.dev/reference/react/cache): `f`, remembering its
/// results. For React Server Components only.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react#cache")]
pub fn cache<F: 'static>(f: F) -> F {
    unreachable!()
}

/// [`cacheSignal`](https://react.dev/reference/react/cacheSignal): aborted
/// when the render that [`cache`] belongs to is done. Server Components only;
/// `None` elsewhere.
#[cfg(react = "19.2")]
#[cfg_attr(rust_js, rust_js::link_name = "react#cacheSignal")]
pub fn cache_signal() -> Option<&'static webapi::AbortSignal> {
    unreachable!()
}

/// [`captureOwnerStack`](https://react.dev/reference/react/captureOwnerStack):
/// which components rendered the current one, in development only.
#[cfg(react = "19.1")]
#[cfg_attr(rust_js, rust_js::link_name = "react#captureOwnerStack")]
pub fn capture_owner_stack() -> Option<String> {
    unreachable!()
}

unsafe extern "Rust" {
    /// React's version, like `"19.3.0"`.
    #[link_name = "react#version"]
    pub safe static VERSION: &'static str;
}

/// What [`use_`] reads: a [`Promise`]'s value, a [`Context`]'s, or, for
/// [`dom::browser`], nothing.
#[cfg(react = "19.0")]
pub trait Usable {
    type Output;
}

#[cfg(react = "19.0")]
impl<T: 'static> Usable for &Promise<T> {
    type Output = &'static T;
}

#[cfg(react = "19.0")]
impl<T: 'static> Usable for &'static LocalKey<Context<T>> {
    type Output = &'static T;
}

/// [`use`](https://react.dev/reference/react/use): a promise's value,
/// suspending until it resolves, or a context's. Unlike a hook, it can be
/// called in a condition or a loop. `use` is a Rust keyword, hence the name.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react#use")]
pub fn use_<U: Usable>(value: U) -> U::Output {
    unreachable!()
}

/// What React gives a root's error callbacks with the error: where it happened.
pub struct ErrorInfo(PhantomData<JsObject>);

impl ErrorInfo {
    /// The components the error went through, as text.
    #[cfg_attr(rust_js, rust_js::link_name = "get componentStack")]
    pub fn component_stack(&self) -> Option<String> {
        unreachable!()
    }
}

/// What React's error callbacks get: whatever was thrown, usually an `Error`.
pub type Error = JsError;
