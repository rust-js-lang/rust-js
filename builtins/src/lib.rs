//! The JS language for rust-js (ADR 0102): what JS has that Rust's `std`
//! doesn't, as ReScript's standard library has it. Its promises, errors and
//! regular expressions, its byte buffers, its JSON, and its global functions,
//! its timers too, as ReScript's has them: every JS runtime has them. What the
//! browser adds is the webapi crate's; what `std` has, rust-js maps itself.
//!
//! It holds declarations only, so it's never compiled to JS: a program calls
//! what it declares, and the calls become plain JS.

// A JS method of several forms, `slice` and `slice_to_end`, is one link
// name of several Rust signatures, as the webapi crate's are.
#![allow(clashing_extern_declarations)]
// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use core::marker::PhantomData;

pub mod atomics;
pub use atomics::{AtomicArray, WaitAsyncResult, WaitAsyncValue, WaitResult, WaitableArray};
pub mod date;
pub use date::Date;
pub mod intl;
mod iterator;
pub use iterator::{Iterator, IteratorResult};
pub mod promise;
pub use promise::{Fulfils, PromiseSettledResult, PromiseWithResolvers};
pub mod proxy;
pub use proxy::{ProxyHandler, RevocableProxy};
pub mod reflect;
pub use reflect::{PropertyDescriptor, PropertyDescriptorMap};
pub mod symbol;
pub use symbol::{Symbol, SymbolFor};
mod typed_arrays;
pub use typed_arrays::*;
mod weak;
pub use weak::*;

/// `import "./App.css";` in the module's JS, for what a module does when
/// it's loaded, as a bundler's CSS does (ADRs 0039 and 0110): written where
/// it's needed, `js::import!("./App.css");`, as stable Rust has no inner
/// attribute of a tool. It's a `const _` rust-js reads, and writes nothing of.
///
/// Of an item, [`import(..)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Operators/import):
/// a promise of it, of its module loaded when it's asked for, a function of
/// the crate's or a binding's of a JS module, which nothing then imports
/// statically (ADR 0304): `let run = js::import!(lint::run).await;` is
/// `const { run } = await import("./lint.js");`, as ReScript's `import(M.x)`.
#[macro_export]
macro_rules! import {
    ($path:literal) => {
        #[cfg_attr(rust_js, rust_js::import = $path)]
        const _: () = ();
    };
    ($item:path) => {
        $crate::import_item($item)
    };
}

/// `js::import!`'s, of an item.
#[doc(hidden)]
#[cfg_attr(rust_js, rust_js::link_name = "import()")]
pub fn import_item<T>(item: T) -> Promise<T> {
    unreachable!()
}

/// The module whose default export is `item`, loaded when it's asked for:
/// `js::import_module!(root::Root)` is `import("./root.js")`, the
/// module's namespace, as React's `lazy` takes it (ADR 0304).
#[macro_export]
macro_rules! import_module {
    ($item:path) => {
        $crate::import_module_of($item)
    };
}

/// `js::import_module!`'s.
#[doc(hidden)]
#[cfg_attr(rust_js, rust_js::link_name = "import(*)")]
pub fn import_module_of<T>(item: T) -> Promise<Module<T>> {
    unreachable!()
}

/// A JS module, loaded, whose default export is a `T`: what
/// [`import_module!`] gives.
pub struct Module<T>(PhantomData<JsObject>, PhantomData<T>);

impl<T> Module<T> {
    /// `module.default`.
    #[cfg_attr(rust_js, rust_js::link_name = "get default")]
    pub fn default(&self) -> T {
        unreachable!()
    }
}

/// What the module runs when it's loaded, its JS's own statements, as a
/// prefetch of what it imports later: `js::on_load! { prefetch(); }` (ADR
/// 0267). rustc checks them as a function's body, which nothing calls.
#[macro_export]
macro_rules! on_load {
    ($($body:tt)*) => {
        #[cfg_attr(rust_js, rust_js::on_load)]
        const _: () = {
            #[cfg_attr(rust_js, rust_js::on_load)]
            #[allow(dead_code)]
            fn on_load() {
                $($body)*
            }
        };
    };
}

/// The module's directive, the first statement of its JS: `"use client";`
/// of a React component a Next.js Server Component renders, written
/// `js::directive!("use client");` (ADR 0192).
#[macro_export]
macro_rules! directive {
    ($directive:literal) => {
        #[cfg_attr(rust_js, rust_js::directive = $directive)]
        const _: () = ();
    };
}

/// The module's default export, a function of its own, which keeps its
/// name too: `js::export_default!(page);` is `export default page;`, what a
/// Next.js route's `page.jsx` has (ADR 0192). The function is named in a
/// `const _`'s `use`, so a plain rustc checks it's there, a generic one's
/// too. Of the function itself, `js::export_default! { fn noop() {} }`,
/// it's declared as the default, `export default function noop() {}`,
/// which rustc sees exported (ADR 0611); of a `const` itself, its value,
/// `export default "19.3.0";` (ADR 0625).
#[macro_export]
macro_rules! export_default {
    ($(#[$meta:meta])* $vis:vis fn $($function:tt)*) => {
        #[cfg_attr(rust_js, rust_js::export_default)]
        #[allow(dead_code)]
        $(#[$meta])* $vis fn $($function)*
    };
    ($(#[$meta:meta])* $vis:vis const $($constant:tt)*) => {
        #[cfg_attr(rust_js, rust_js::export_default)]
        #[allow(dead_code)]
        $(#[$meta])* $vis const $($constant)*
    };
    ($function:path) => {
        #[cfg_attr(rust_js, rust_js::export_default)]
        const _: () = {
            #[allow(unused_imports)]
            use $function as _;
        };
    };
}

/// Gone (ADR 0435): React's names, a component's props and a hook, are
/// camelCase in JS, and a crate's own keep Rust's, with no setting.
#[macro_export]
macro_rules! camel_case {
    ($($any:tt)*) => {
        ::core::compile_error!(
            "`js::camel_case!` is gone: a component's props and a hook are React's names, camelCase, in JS, and the rest Rust's (ADR 0435); remove it"
        );
    };
}

/// Any JS object. Every type here and in webapi holds a `PhantomData` of
/// it, which is how rust-js knows it's a JS object, and so may a program's
/// own: `pub struct EditorView(PhantomData<JsObject>);` (ADR 0111). A program
/// never has one by value, only a reference a binding gives: its field is
/// private, so nothing makes one, and it's neither `Send` nor `Sync`. As a
/// value, TypeScript's `object`, what `Kind::Object` holds: a key of a weak
/// collection, and an [`Unknown`] it derefs to, whose properties [`get`]
/// reads (ADR 0677).
#[cfg_attr(rust_js, rust_js::js_object)]
#[cfg_attr(rust_js, rust_js::types = "object")]
pub struct JsObject(PhantomData<*mut ()>);

/// An object is a value of any shape, as TypeScript's `object` is an
/// `unknown`: the object itself.
impl core::ops::Deref for JsObject {
    type Target = Unknown;

    fn deref(&self) -> &Unknown {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const Unknown) }
    }
}

/// A JS value of unknown shape (ADR 0225), as TypeScript's `unknown` and
/// ReScript's are: what `JSON.parse` gives, or a binding's of `any`. Never
/// `undefined` nor `null`, which an `Option<&Unknown>` is `None` of (ADR
/// 0030). [`classify`] tells what it is, and [`get`] and [`set`] reach a
/// property of it by name.
#[cfg_attr(rust_js, rust_js::types = "unknown")]
pub struct Unknown(PhantomData<JsObject>);

/// `a === b`: the same object, or the same value, JS's strict equality, as
/// TypeScript compares two of `unknown`; `NaN` isn't itself, as an `f64`
/// isn't, so it's `PartialEq` alone (ADR 0612).
impl PartialEq for Unknown {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// JS's relational comparison, as upstream code orders values of any
/// shape: `Less` where `a < b`, `Greater` where `a > b`, `Equal` where `a
/// === b`, as `==` is (ADR 0612), else none, `NaN`'s or an object's and a
/// number's. `a < b` and `a > b` are JS's own; `a <= b` is `a < b || a ===
/// b`, not JS's `<=`, which coerces `"1" <= 1` true (ADR 0931).
impl PartialOrd for Unknown {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self == other).then_some(std::cmp::Ordering::Equal)
    }
}

impl Unknown {
    /// [`value.hasOwnProperty(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/hasOwnProperty):
    /// the value's own `hasOwnProperty` called, as `value.hasOwnProperty(key)`
    /// asks it: whether it has a property of that name of its own, or what
    /// a `hasOwnProperty` of its own says; of one with none, an
    /// `Object.create(null)`, JS's `TypeError`, a panic. Where no value's
    /// own may answer, [`object::has_own_property`]`.call(value, key)` is
    /// `Object.prototype`'s (ADR 0615, amended).
    #[cfg_attr(rust_js, rust_js::link_name = "hasOwnProperty")]
    pub fn has_own_property<K: PropertyKey + ?Sized>(&self, key: &K) -> bool {
        unreachable!()
    }
}

/// What a JS value is, as JS's `typeof`, `=== null` and `Array.isArray`
/// tell it: each variant's value is the value itself (ADR 0214), a `match`
/// of them a `switch (typeof value)` (ADR 0668). An `Option<&Unknown>`'s
/// `None` is `Undefined` or `Null` (ADR 0676). An array's items may be
/// `undefined` or `null`; a function is a `Function`; what's left, an
/// object neither `null` nor an array, an `Object`, whose properties
/// [`get`] reads.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Kind<'a> {
    String(&'a str),
    Number(f64),
    /// A `bigint`, as an `i64` is one (ADR 0086): one wider isn't wrapped.
    BigInt(i64),
    Bool(bool),
    /// A symbol, `typeof value === "symbol"`.
    Symbol(&'a Symbol),
    /// A function, `typeof value === "function"`, TypeScript's `Function`:
    /// called of nothing, what it gives is any value (ADR 0700).
    Function(&'a dyn Fn() -> Option<&'static Unknown>),
    Array(&'a [Option<&'a Unknown>]),
    /// `undefined`: `value === undefined`.
    #[cfg_attr(rust_js, rust_js::undefined)]
    Undefined,
    /// `null`: `value === null`, which `typeof` says is an `"object"`.
    #[cfg_attr(rust_js, rust_js::null)]
    Null,
    /// What the others aren't: `typeof value === "object"`, neither `null`
    /// nor an array (ADR 0668); TypeScript's `object`, a weak collection's
    /// key, which reads as the `Unknown` it is (ADR 0677).
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Object(&'a JsObject),
}

impl<'a> Kind<'a> {
    /// The object it is, an `Object`'s or an `Array`'s: what `typeof value
    /// === "object" && value !== null` takes, as one value, TypeScript's
    /// `object` it narrows to, a weak collection's key. `None` of the
    /// others. `if let Some(object) = js::classify(value).object()` is that
    /// `if`, its `object` the value itself (ADR 0910).
    #[cfg_attr(rust_js, rust_js::link_name = "typeof object")]
    pub fn object(self) -> Option<&'a JsObject> {
        unreachable!()
    }
}

/// JS's [`Array`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Array)
/// functions, for a value an API takes or gives.
pub mod array {
    use super::*;

    /// [`Array.isArray(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Array/isArray):
    /// whether `value` is an array, as [`classify`]'s `Kind::Array` tells.
    #[cfg_attr(rust_js, rust_js::link_name = "Array.isArray")]
    pub fn is_array(value: Option<&Unknown>) -> bool {
        unreachable!()
    }

    /// [`array.splice(start, deleteCount)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Array/splice):
    /// of any value an array, `deleteCount` of its items from `start`
    /// removed in place, as JS's `splice` does: each a number, as JS's is,
    /// made a whole one, a negative `start` counted from the end, past it
    /// none; what it removed. Of a value with no `splice`, JS's
    /// `TypeError`, a panic (ADR 0967, amended). An array Rust owns is a
    /// `Vec`, whose `remove` and `drain` say this.
    #[cfg_attr(rust_js, rust_js::link_name = "splice")]
    #[allow(unused_variables)]
    pub fn splice(this: &Unknown, start: f64, delete_count: f64) -> Vec<Option<&'static Unknown>> {
        unreachable!()
    }

    /// [`array.join(separator)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Array/join):
    /// the text of each of any values, as JS's `String()` gives it, between
    /// `separator`s, `undefined` and `null` none: React's
    /// `prevDeps.join(', ')`. Of a symbol, JS's `TypeError`, a panic. Text
    /// Rust owns is joined by `[String]::join`.
    #[cfg_attr(rust_js, rust_js::link_name = "join")]
    #[allow(unused_variables)]
    pub fn join(this: &[Option<&Unknown>], separator: &str) -> String {
        unreachable!()
    }
}

/// What `value` is: `match classify(value) { Kind::String(s) => .., .. }`,
/// a `typeof` of it in JS; of `None`, `Undefined` or `Null`, and of one
/// that's there, `classify(Some(value))` (ADR 0676). The value itself.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn classify(this: Option<&Unknown>) -> Kind<'_> {
    unreachable!()
}

/// `value` as a JS value of any shape, which it is: what's never `undefined`
/// nor `null` ([`AnyShape`]), text or a number, say, or a function, as any
/// value is TypeScript's `unknown`. The value itself:
/// `js::set(js::unknown(&disabled_log), "__reactDisabledLog", true)` is
/// `disabledLog.__reactDisabledLog = true` (ADR 0673).
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn unknown<T: AnyShape<M> + ?Sized, M>(this: &T) -> &Unknown {
    unreachable!()
}

/// `value`, given by value, as a JS value of any shape: an element, a
/// handle, or an object of its fields, made where it's given, or a
/// closure, which [`unknown`] would borrow only for its block. The value
/// itself, kept as long as JS has it.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn unknown_of<T: AnyShape<M> + 'static, M>(this: T) -> &'static Unknown {
    unreachable!()
}

/// `value`, a type parameter's, given by value, as any JS value, which it
/// is: TypeScript's `any`, `None` where it's `undefined` or `null`, as an
/// `Option`'s `None` is. What keeps values of types a program erases, as
/// React's value stack keeps each cursor's, `const valueStack: Array<any>`,
/// `valueStack[index] = js::any_of(previous)`; [`cast`] gives it back as
/// what it was. The value itself, kept as long as JS has it (ADR 0909). Of
/// a value of a type that's known, a compile error: [`unknown_of`] gives
/// one that's always there, and `value.map(js::unknown_of)` an `Option`.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn any_of<T: 'static>(this: T) -> Option<&'static Unknown> {
    unreachable!()
}

/// What [`unknown`] takes, a JS value of any shape, never `undefined` nor
/// `null`: a [`Defined`] value, or a function, of whatever it takes, a
/// function item, a closure, a `fn` pointer or a `dyn Fn`, each a JS
/// function, which is an object (ADR 0673), or an `Rc` of one, a shared
/// function, the function itself (ADR 0965). `M` is which one, rustc's to
/// infer: no program names it. A function isn't `Defined` only because
/// Rust can't say each is: a function item's type has no name, and an
/// impl for each `Fn` would be one for `&T` too.
pub trait AnyShape<M: ?Sized> {}

impl<T: Defined + ?Sized> AnyShape<()> for T {}

/// Any function, of whatever it takes and gives, as each is TypeScript's
/// `Function`: a function item, a closure, a `fn` pointer or a `dyn Fn` of
/// any signature, a JS function of JS's or of the program's (ADR 0740).
/// What [`function`]'s members, `Function.prototype`'s, are read of. `M`
/// is which one, rustc's to infer, as [`AnyShape`]'s is.
pub trait AnyFunction<M: ?Sized> {}

/// A function of each arity up to 16, its marker the `fn` pointer of its
/// signature: of any shape, and any function.
macro_rules! functions {
    ($(($($a:ident),*))*) => {
        $(
            impl<F: FnOnce($($a),*) -> R + ?Sized, R, $($a),*> AnyShape<fn($($a),*) -> R> for F {}
            impl<F: FnOnce($($a),*) -> R + ?Sized, R, $($a),*> AnyShape<std::rc::Rc<fn($($a),*) -> R>>
                for std::rc::Rc<F>
            {
            }
            impl<F: FnOnce($($a),*) -> R + ?Sized, R, $($a),*> AnyFunction<fn($($a),*) -> R> for F {}
        )*
    };
}

/// What every JS function has, `Function.prototype`'s, of a function of
/// any signature, [`AnyFunction`], as TypeScript's `Function` types them
/// (ADR 0740): `function::name(f)` is `f.name`.
pub mod function {
    use super::AnyFunction;

    /// [`f.name`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Function/name):
    /// its name, `""` of an anonymous one, as TypeScript's `string` has it.
    #[cfg_attr(rust_js, rust_js::link_name = "get name")]
    pub fn name<F: AnyFunction<M> + ?Sized, M>(this: &F) -> String {
        unreachable!()
    }

    /// [`f.length`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Function/length):
    /// how many parameters it declares before its first with a default
    /// or its rest.
    #[cfg_attr(rust_js, rust_js::link_name = "get length")]
    pub fn length<F: AnyFunction<M> + ?Sized, M>(this: &F) -> u32 {
        unreachable!()
    }

    /// [`Function.prototype.toString.call(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Function/toString):
    /// its source, as JS keeps it, `function () { [native code] }` of a
    /// host's own; what no function of its own `toString` changes.
    #[cfg_attr(rust_js, rust_js::link_name = "Function.prototype.toString.call")]
    pub fn to_string<F: AnyFunction<M> + ?Sized, M>(f: &F) -> String {
        unreachable!()
    }

    /// [`f.call(thisArg, ..args)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Function/call):
    /// `f` called with `this_arg` as its `this`, any value, `None` of
    /// `undefined`, and `args`, a rest parameter of any values, as
    /// TypeScript types it: what it gives, any value. `nodeRequire.call(module,
    /// "timers")` of `function::call(node_require, Some(&module), ("timers",))`.
    #[cfg_attr(rust_js, rust_js::link_name = "call")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn call<F: AnyFunction<M> + ?Sized, M>(
        this: &F,
        this_arg: Option<&dyn super::IntoUnknown>,
        args: impl super::Rest<dyn super::IntoUnknown>,
    ) -> Option<&'static super::Unknown> {
        unreachable!()
    }

    /// [`f.apply(thisArg, argArray)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Function/apply):
    /// `f` called with `this_arg` as its `this` and the items of
    /// `arg_array`, an array, as its arguments, each any value, as
    /// TypeScript types them: `None` of `undefined`, `Some(&Kind::Null)` of
    /// `null`, and no arguments of `None`. What it gives, any value.
    /// `fn.apply(null, args)` of `function::apply(r#fn, Some(&Kind::Null),
    /// Some(&args))`.
    #[cfg_attr(rust_js, rust_js::link_name = "apply")]
    pub fn apply<F: AnyFunction<M> + ?Sized, M>(
        this: &F,
        this_arg: Option<&dyn super::IntoUnknown>,
        arg_array: Option<&dyn super::IntoUnknown>,
    ) -> Option<&'static super::Unknown> {
        unreachable!()
    }

    /// [`f.bind(thisArg, ...args)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Function/bind):
    /// a function that calls `f` with `this_arg` as its `this`, any value,
    /// `None` of `undefined`, `Some(&Kind::Null)` of `null`, and `args`, a
    /// tuple of `f`'s first arguments, before what it's given, as
    /// TypeScript types it (ADR 0907, amended). Of none, `()`, a function
    /// of `f`'s own type, `OmitThisParameter<T>`, `f` one that's `Copy`, a
    /// `fn` pointer or a `&dyn Fn`: React keeps a frame's function so,
    /// `function::bind(CALL_COMPONENT.react_stack_bottom_frame,
    /// Some(&CALL_COMPONENT), ())`. Of some, a `Box<dyn Fn(..) -> R>` of the
    /// parameters `f` has left, which lives as long as what it keeps: each
    /// argument a value of a `Copy` type, or what a parameter borrows, lent
    /// to each call ([`Keeps`]), as a closure `move |action|
    /// dispatch(&fiber, action)` lends what it keeps. React's
    /// `dispatchSetState.bind(null, fiber, queue)` of
    /// `function::bind(dispatch_set_state, Some(&Kind::Null), (fiber, queue))`.
    #[cfg_attr(rust_js, rust_js::link_name = "bind")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn bind<'l, F, M, L>(this: F, this_arg: Option<&dyn super::IntoUnknown>, args: L) -> <(F, M) as Bound<'l, L>>::Rest
    where
        F: AnyFunction<M> + 'l,
        (F, M): Bound<'l, L>,
        L: super::Rest<dyn super::IntoUnknown>,
    {
        unreachable!()
    }

    /// [`function (a, b) { .. }`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Operators/function):
    /// a JS function of `f`, a closure given the function's `this`, any
    /// value, `None` of `undefined` or `null`, and a tuple of what it's
    /// called with, `(a, b)`, `()` of nothing (ADR 0972). Its `this` is
    /// what JS calls it on: `function::call(f, Some(&instance), ())`'s
    /// `instance`, `object.f()`'s `object`, `None` of a call of it alone.
    /// React's `update.callback = function callback() { .. this .. }` of
    /// `update.callback.set(Some(js::unknown_of(function::with_this(move
    /// |this, ()| { .. }))))`. A closure, so what it captures is Rust's to
    /// check, as any closure's is; called from Rust, `f(a, b)`, its `this`
    /// is `None`.
    #[cfg_attr(rust_js, rust_js::link_name = "function")]
    pub fn with_this<'l, F, A, R>(this: F) -> <A as Parameters<'l, R>>::Function
    where
        F: Fn(Option<&'static super::Unknown>, A) -> R + 'l,
        A: Parameters<'l, R>,
    {
        unreachable!()
    }

    /// What a function [`with_this`] makes is, given a tuple of its
    /// parameters, `Self`, and giving `R`: of `(A, B)`, a `Box<dyn Fn(A, B)
    /// -> R>`, which lives as long as what its closure keeps, `'l`. Of each
    /// tuple of up to 12. No program names it.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` isn't a tuple of the parameters a JS function takes",
        note = "a function `with_this` makes is given its `this` and a tuple of its parameters: `|this, (a, b)|`, `|this, (a,)|` of one, `|this, ()|` of none"
    )]
    pub trait Parameters<'l, R> {
        /// The JS function.
        type Function;
    }

    /// `Parameters` of each tuple of up to 12.
    macro_rules! parameters {
        ($(($($a:ident),*))*) => {
            $(
                impl<'l, R, $($a),*> Parameters<'l, R> for ($($a,)*) {
                    type Function = Box<dyn Fn($($a),*) -> R + 'l>;
                }
            )*
        };
    }

    parameters! {
        ()
        (A)
        (A, B)
        (A, B, C)
        (A, B, C, D)
        (A, B, C, D, E)
        (A, B, C, D, E, F1)
        (A, B, C, D, E, F1, G)
        (A, B, C, D, E, F1, G, H)
        (A, B, C, D, E, F1, G, H, I)
        (A, B, C, D, E, F1, G, H, I, J)
        (A, B, C, D, E, F1, G, H, I, J, K)
        (A, B, C, D, E, F1, G, H, I, J, K, L)
    }

    /// What a function `F`, of the signature `M`, a `fn` pointer's, is with
    /// its first arguments `L` bound, [`bind`]'s: of none, `F` itself, and
    /// of some, a function of the parameters it has left, which lives as
    /// long as what it keeps, `'l`. Of each signature of up to 16
    /// parameters and each count of them bound. No program names it.
    #[diagnostic::on_unimplemented(
        message = "`{L}` aren't the first arguments a function `{Self}` takes",
        note = "a function's first arguments are bound as a tuple, `(a, b)`, `(a,)` of one and `()` of none, each of a `Copy` type or what its parameter borrows; bound of none, the function is `Copy`"
    )]
    pub trait Bound<'l, L> {
        /// The function bound.
        type Rest;
    }

    impl<F: Copy, M> Bound<'_, ()> for (F, M) {
        type Rest = F;
    }

    /// What a bound function keeps of an argument it's given first for a
    /// parameter of `P`: a value of a `Copy` type, which each call is given
    /// as it is, or what a parameter borrows, `Rc<Fiber>` of a
    /// `&Rc<Fiber>`, or of an `Option<&Rc<Fiber>>` an `Option<Rc<Fiber>>`,
    /// which each call is lent, as a closure lends what it keeps,
    /// `errors.as_ref()`. In JS each is the value itself.
    #[diagnostic::on_unimplemented(
        message = "a bound function can't keep a `{Self}` for a parameter of `{P}`",
        note = "it keeps a value of a `Copy` type, or what a parameter borrows: `fiber` of a `&Rc<Fiber>`, `errors` of an `Option<&Rc<Errors>>`"
    )]
    pub trait Keeps<P> {}

    impl<T: Copy> Keeps<T> for T {}
    impl<T> Keeps<&T> for T {}
    impl<T> Keeps<Option<&T>> for Option<T> {}

    /// `Bound` of one signature, each count of its first parameters bound
    /// but none: `$k`, each kept as an `$x`, and `$r` left.
    macro_rules! bound {
        (@split [$($k:ident $x:ident),+] [$($r:ident),*]) => {
            impl<'l, F, $($k, $x: Keeps<$k> + 'l,)* $($r,)* Ret> Bound<'l, ($($x,)*)> for (F, fn($($k,)* $($r),*) -> Ret) {
                type Rest = Box<dyn Fn($($r),*) -> Ret + 'l>;
            }
        };
        (@walk [$($k:ident $x:ident),*] [] []) => {
            bound!(@split [$($k $x),*] []);
        };
        (@walk [] [$r:ident $($rs:ident)*] [$y:ident $($ys:ident)*]) => {
            bound!(@walk [$r $y] [$($rs)*] [$($ys)*]);
        };
        (@walk [$($k:ident $x:ident),+] [$r:ident $($rs:ident)*] [$y:ident $($ys:ident)*]) => {
            bound!(@split [$($k $x),+] [$r $(, $rs)*]);
            bound!(@walk [$($k $x,)+ $r $y] [$($rs)*] [$($ys)*]);
        };
        ($([$($p:ident)*] [$($q:ident)*])*) => {
            $(bound!(@walk [] [$($p)*] [$($q)*]);)*
        };
    }

    bound! {
        [A] [XA]
        [A B] [XA XB]
        [A B C] [XA XB XC]
        [A B C D] [XA XB XC XD]
        [A B C D E] [XA XB XC XD XE]
        [A B C D E F1] [XA XB XC XD XE XF]
        [A B C D E F1 G] [XA XB XC XD XE XF XG]
        [A B C D E F1 G H] [XA XB XC XD XE XF XG XH]
        [A B C D E F1 G H I] [XA XB XC XD XE XF XG XH XI]
        [A B C D E F1 G H I J] [XA XB XC XD XE XF XG XH XI XJ]
        [A B C D E F1 G H I J K] [XA XB XC XD XE XF XG XH XI XJ XK]
        [A B C D E F1 G H I J K L] [XA XB XC XD XE XF XG XH XI XJ XK XL]
        [A B C D E F1 G H I J K L M] [XA XB XC XD XE XF XG XH XI XJ XK XL XM]
        [A B C D E F1 G H I J K L M N] [XA XB XC XD XE XF XG XH XI XJ XK XL XM XN]
        [A B C D E F1 G H I J K L M N O] [XA XB XC XD XE XF XG XH XI XJ XK XL XM XN XO]
        [A B C D E F1 G H I J K L M N O P] [XA XB XC XD XE XF XG XH XI XJ XK XL XM XN XO XP]
    }
}

functions! {
    ()
    (A)
    (A, B)
    (A, B, C)
    (A, B, C, D)
    (A, B, C, D, E)
    (A, B, C, D, E, F1)
    (A, B, C, D, E, F1, G)
    (A, B, C, D, E, F1, G, H)
    (A, B, C, D, E, F1, G, H, I)
    (A, B, C, D, E, F1, G, H, I, J)
    (A, B, C, D, E, F1, G, H, I, J, K)
    (A, B, C, D, E, F1, G, H, I, J, K, L)
    (A, B, C, D, E, F1, G, H, I, J, K, L, M)
    (A, B, C, D, E, F1, G, H, I, J, K, L, M, N)
    (A, B, C, D, E, F1, G, H, I, J, K, L, M, N, O)
    (A, B, C, D, E, F1, G, H, I, J, K, L, M, N, O, P)
}

/// Whether `value` is [truthy](https://developer.mozilla.org/docs/Glossary/Truthy),
/// as JS's `if (value)` asks: `None`, `""`, `0`, `NaN` and `false` aren't,
/// all else is.
#[cfg_attr(rust_js, rust_js::link_name = "!!")]
#[allow(unused_variables)]
pub fn truthy<T: Defined + ?Sized>(this: Option<&T>) -> bool {
    unreachable!()
}

/// `a === b`, JS's strict equality of any two values, as TypeScript's
/// `===` compares them: the same object, or the same primitive, `NaN` not
/// itself, and `null` not `undefined`, so two `None`s differ where JS has
/// them so. `==` is Rust's, two `None`s equal, whichever each is (ADR
/// 0612); this is JS's, for what a port of JS's `===` compares (ADR 0721).
#[cfg_attr(rust_js, rust_js::link_name = "===")]
#[allow(unused_variables)]
pub fn strict_equals(this: impl IntoUnknown, other: impl IntoUnknown) -> bool {
    unreachable!()
}

/// `a == b`, JS's loose equality of any two values, which coerces them as
/// JS does: `"0.0" == 0`, `"1" == 1`, `null == undefined` hold. For a port
/// of JS that compares so on purpose, React DOM's `node.value != value`,
/// which leaves `"0.0"` as typed where the value is `0`; any other
/// comparison is Rust's `==`, or [`strict_equals`]'s `===` (ADR 1012).
#[cfg_attr(rust_js, rust_js::link_name = "==")]
#[allow(unused_variables)]
pub fn loose_equals(this: impl IntoUnknown, other: impl IntoUnknown) -> bool {
    unreachable!()
}

/// JS's `null`, as an `Option` of any type: `None` to Rust, as either of
/// JS's `None`s is (ADR 0030), and `null` where it's written, as a field
/// TypeScript types `T | null` starts: React's fiber's `memoizedState:
/// null`, of `memoized_state: Cell::new(js::null())`, and `_debugStack:
/// null`, of `debug_stack: Cell::new(js::null())`. What's stored there
/// later is stored as it is, `undefined` too, where a
/// `#[rust_js::nullable]` field would store `null` for every `None` (ADR
/// 0935).
#[cfg_attr(rust_js, rust_js::link_name = "null", rust_js::nullable)]
pub const fn null<T>() -> Option<T> {
    None
}

/// `value` as a `T`, unchecked, as TypeScript's `any` is given a type: the
/// value itself.
///
/// # Safety
///
/// `value` must be what JS has of a `T`: a JSON object of strings is a
/// `&Dict<String>`, a string or `undefined` an `Option<String>`. A value
/// that may be `undefined` is cast to an `Option`, never to what's in one:
/// `Some(cast::<fn()>(v))` of an `undefined` is `undefined`, which is the
/// `Option`'s `None`, as JS has `Some` of a function as the function
/// itself; `cast::<Option<fn()>>(v)` says so.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub unsafe fn cast<T>(this: Option<&Unknown>) -> T {
    unreachable!()
}

/// What JS takes as text where it wants text, which TypeScript's `string`
/// is given: text, or any value, which JS makes text, `"undefined"` of
/// `None` (ADR 0310); a number as JS writes it, `1e+21`.
pub trait ToText {
    /// What [`concat_text`] gives of it: its text, of text or a number; of
    /// a value of any shape, a `Result`, as JS's `+` throws of a symbol and
    /// of an object whose `valueOf` throws (ADR 0639).
    type Text;
}

impl ToText for &str {
    type Text = String;
}
impl ToText for &String {
    type Text = String;
}
impl ToText for String {
    type Text = String;
}
impl ToText for &Unknown {
    type Text = Result<String, &'static JsError>;
}
impl ToText for &JsObject {
    type Text = Result<String, &'static JsError>;
}
impl ToText for Option<&Unknown> {
    type Text = Result<String, &'static JsError>;
}
impl ToText for f64 {
    type Text = String;
}

/// [`parseInt(text, radix)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/parseInt):
/// the integer `text` starts with, `NaN` of none: `"4x"` is 4, where Rust's
/// `str::parse` is an `Err`; in base `radix`, or 16 after a `0x`, else 10.
/// Of any value, of its text.
#[cfg_attr(rust_js, rust_js::link_name = "parseInt")]
#[allow(unused_variables)]
pub fn parse_int<T: ToText>(text: T, radix: Option<u32>) -> f64 {
    unreachable!()
}

/// [`parseFloat(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/parseFloat):
/// the number `text` starts with, `NaN` of none. Of any value, of its text.
#[cfg_attr(rust_js, rust_js::link_name = "parseFloat")]
#[allow(unused_variables)]
pub fn parse_float<T: ToText>(text: T) -> f64 {
    unreachable!()
}

/// `"" + value`: any value as text, as JS's `+` makes it, `result += value`
/// say: of an object, by its `valueOf` first, where `String(value)` asks its
/// `toString` (ADR 0310). Of a value of any shape, a `Result`: it throws of
/// a symbol, which `String` names, and of an object whose `valueOf` throws,
/// an `Err` (ADR 0639); of text or a number, its text.
#[cfg_attr(rust_js, rust_js::link_name = "\"\" +")]
#[allow(unused_variables)]
pub fn concat_text<T: ToText>(this: T) -> T::Text {
    unreachable!()
}

/// Any value as JS's template shows it, `${value}`: `format!("{}-{index}",
/// js::shown(msg))` is `${msg}-${index}` (ADR 0312). The value itself.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn shown<T: ToText>(this: T) -> &'static Shown {
    unreachable!()
}

/// What [`shown`] gives: a value, whose text is what JS's template makes it.
pub struct Shown(PhantomData<JsObject>);

impl core::fmt::Display for Shown {
    /// The value itself, in its template.
    #[cfg_attr(rust_js, rust_js::link_name = "this")]
    #[allow(unused_variables)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unreachable!()
    }
}

/// [`typeof value`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Operators/typeof):
/// what JS says it is, as text, `"undefined"` of `None` (ADR 0310).
#[cfg_attr(rust_js, rust_js::link_name = "typeof")]
#[allow(unused_variables)]
pub fn type_of(this: Option<&Unknown>) -> String {
    unreachable!()
}

/// [`typeof X !== "undefined"`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Operators/typeof#interaction_with_undeclared_and_uninitialized_variables):
/// whether the host has `binding`'s global, a JS function's binding,
/// `is_defined(webapi::message_channel::new)`, or a static's, given as
/// itself, `is_defined(js::symbol::to_string_tag)`, where reading one it
/// lacks would throw (ADRs 0597, 0645); of a path of them, each a method of what the
/// one before gives, each part (ADR 0624). Rust is told nothing: the
/// binding is still there to call, which throws where the host hasn't it.
#[cfg_attr(rust_js, rust_js::link_name = "typeof !== \"undefined\"")]
#[allow(unused_variables)]
pub fn is_defined<B: Copy>(binding: B) -> bool {
    unreachable!()
}

/// [`typeof X === "function"`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Operators/typeof):
/// whether `binding`'s global is a function the host can call, of what
/// [`is_defined`] is given, a JS function's binding, a static, or a path of
/// them, each part before the last asked whether it's there (ADR 0942).
/// Rust is told nothing: the binding is still there to call.
#[cfg_attr(rust_js, rust_js::link_name = "typeof === \"function\"")]
#[allow(unused_variables)]
pub fn is_function<B: Copy>(binding: B) -> bool {
    unreachable!()
}

/// [`String(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/String):
/// any value as text, as JS makes it, `result += value` say: `"undefined"`
/// of `None`, `"[object Object]"` of an object.
#[cfg_attr(rust_js, rust_js::link_name = "String")]
#[allow(unused_variables)]
pub fn string(value: Option<&Unknown>) -> String {
    unreachable!()
}

/// `value[key]`: a property of any value, by its name, as TypeScript's
/// `any` is read; `None` where it's `undefined` or `null`, as where there's
/// none. Of `undefined` or `null` itself, a `None`, JS's `TypeError`, a
/// panic, as JS throws it (ADR 0776).
#[cfg_attr(rust_js, rust_js::link_name = "get []")]
#[allow(unused_variables)]
pub fn get<K: PropertyKey + ?Sized>(this: impl IntoUnknown, key: &K) -> Option<&'static Unknown> {
    unreachable!()
}

/// `delete value[key]`: the object's own property of that name gone, as
/// JS's `delete` makes it, and whether it is, `false` of one that can't
/// be, which JS's strict mode throws a `TypeError` of, a panic (ADR 0697).
/// Of any value, as TypeScript's `delete` is of an `any`'s; of `undefined`
/// or `null`, a `None`, JS's `TypeError`, a panic (ADR 0776).
#[cfg_attr(rust_js, rust_js::link_name = "delete []")]
#[allow(unused_variables)]
pub fn delete<K: PropertyKey + ?Sized>(this: impl IntoUnknown, key: &K) -> bool {
    unreachable!()
}

/// `new value(...arguments)`: what JS's `new` makes of a constructor. Of a
/// typed [`Class`], `Class<fn(A, B) -> T>`, its arguments a tuple of what
/// its constructor takes, `(a, b)`, each checked as a call's is, and what
/// it makes a `&'static T`: React's `new SyntheticEventCtor(reactName, ..)`
/// of `js::new(ctor, (react_name, ..))`. Of an untyped `Class`, a binding's
/// global class or a crate's, `js::class::<T>()`, or of a value of any
/// shape, a function JS gives, as TypeScript's `new` is of an `any`'s, its
/// arguments a rest parameter's, a tuple of them, `()` of none, or a slice
/// spread, and what it makes a value of any shape (ADR 0695, amended). A
/// constructor the program writes is held as a `Class`, [`Class::of`]: a
/// `js::unknown(&fake)` of one is refused, naming it. Of what's no
/// constructor, an arrow, JS's `TypeError`, a panic.
#[cfg_attr(rust_js, rust_js::link_name = "new this()")]
#[cfg_attr(rust_js, rust_js::variadic)]
#[allow(unused_variables)]
pub fn new<C: Construct<A> + ?Sized, A>(this: &C, arguments: A) -> C::Instance {
    unreachable!()
}

/// What [`new`] constructs, given `A`, and what that makes: a typed
/// [`Class`] of the tuple of its constructor's parameters, an untyped one,
/// or a value of any shape of rest arguments. No program names it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't a constructor `new` is given `{A}` of",
    note = "a `Class<fn(A, B) -> T>` is given a tuple of its parameters, `(a, b)`, `(a,)` of one and `()` of none; a constructor the program writes is held as `Class::of(&f)`"
)]
pub trait Construct<A> {
    /// What `new` of it makes.
    type Instance;
}

impl<A: Rest<dyn IntoUnknown>> Construct<A> for Unknown {
    type Instance = &'static Unknown;
}

impl<A: Rest<dyn IntoUnknown>> Construct<A> for Class {
    type Instance = &'static Unknown;
}

/// `Construct` of each typed `Class` of up to 12 parameters.
macro_rules! constructs {
    ($(($($a:ident),*))*) => {
        $(
            impl<T: 'static, $($a),*> Construct<($($a,)*)> for Class<fn($($a),*) -> T> {
                type Instance = &'static T;
            }
            impl<R: Made, $($a),*> Signature for fn($($a),*) -> R {
                type Class = fn($($a),*) -> R::Instance;
            }
        )*
    };
}

constructs! {
    ()
    (A)
    (A, B)
    (A, B, C)
    (A, B, C, D)
    (A, B, C, D, E)
    (A, B, C, D, E, F)
    (A, B, C, D, E, F, G)
    (A, B, C, D, E, F, G, H)
    (A, B, C, D, E, F, G, H, I)
    (A, B, C, D, E, F, G, H, I, J)
    (A, B, C, D, E, F, G, H, I, J, K)
    (A, B, C, D, E, F, G, H, I, J, K, L)
}

/// What [`Class::of`] makes of a function's signature, `fn(A, B) -> R`: the
/// class's, `fn(A, B) -> T`, `T` what `new` of it makes ([`Made`]). No
/// program names it.
#[diagnostic::on_unimplemented(
    message = "a function of `{Self}` is no constructor `Class::of` types",
    note = "a constructor takes up to 12 parameters and gives `this`, a `&'static T`, or nothing"
)]
pub trait Signature {
    /// The class's signature.
    type Class;
}

/// What `new` of a function that gives `Self` makes, as JS's `new` does: what
/// it gives where that's an object, `this` of a `&'static T`, its own
/// `return this;`; a value of any shape where it gives nothing, `()`, or an
/// `Option`, `this` itself where it's `None`. No program names it.
#[diagnostic::on_unimplemented(
    message = "a constructor that gives `{Self}` makes no instance `new` can type",
    note = "a constructor gives `this`, a `&'static T`, or nothing"
)]
pub trait Made {
    /// What `new` of it makes.
    type Instance;
}

impl<T> Made for &'static T {
    type Instance = T;
}

impl Made for () {
    type Instance = Unknown;
}

impl<T> Made for Option<T> {
    type Instance = Unknown;
}

/// `throw value`: JS's `throw` of a value JS has, an `Error`, a promise,
/// any value, or `undefined` of `None`, as React's `throw
/// payload._result;` (ADR 0760). The one way to throw what's already a
/// JS value; a new `Error` is `panic!`'s (ADR 0706), and what
/// `catch_unwind` caught is thrown again by `resume_unwind`. What it throws
/// unwinds as any JS throw: `catch_unwind` catches it, its payload's
/// `downcast_ref::<&JsError>()` the value thrown (ADR 0710).
#[cfg_attr(rust_js, rust_js::link_name = "throw")]
#[allow(unused_variables)]
pub fn throw(this: Option<&Unknown>) -> ! {
    unreachable!()
}

/// A JS class's constructor function, `Component`, which [`class`] gives
/// of a struct marked `#[rust_js::class]`, or of a binding's class: its
/// `prototype`, which a port sets as JS does, `ComponentDummy.prototype =
/// Component.prototype` (ADR 0761), and what [`new`] makes one of. Typed by
/// its constructor's signature, `Class<fn(A, B) -> T>`, its parameters and
/// what `new` makes, where [`Class::of`] makes it of a function the program
/// writes; of none, `Class`, any constructor's.
pub struct Class<S = AnyConstructor>(PhantomData<(JsObject, S)>);

/// What an untyped [`Class`] is of: any constructor, given any arguments,
/// making a value of any shape.
pub enum AnyConstructor {}

unsafe impl<S> Defined for Class<S> {}

impl<S> Class<S> {
    /// A function the program writes as the constructor it is, typed by
    /// its signature: `function::with_this(move |this, (a, b)| { .. this
    /// })`'s, closed over what it captures, or a `fn` item's, `&fake`
    /// (ADRs 0972, 0696, amended). The function itself, which `new` makes
    /// a `&'static T` of where it gives `this`, a `&'static T`; a value of
    /// any shape where it gives nothing or an `Option`. React's `function
    /// SyntheticBaseEvent(..) { this._reactName = reactName; .. return this; }`
    /// of `Class::of(function::with_this(move |this, (react_name, ..)| ..))`.
    #[cfg_attr(rust_js, rust_js::link_name = "this")]
    #[allow(unused_variables)]
    pub fn of<F: AnyFunction<M>, M: Signature<Class = S>>(this: F) -> &'static Class<S> {
        unreachable!()
    }

    /// [`C.prototype`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Function/prototype):
    /// what each instance `new C(..)` makes inherits from.
    #[cfg_attr(rust_js, rust_js::link_name = "get prototype")]
    pub fn prototype(&self) -> &'static Unknown {
        unreachable!()
    }

    /// `C.prototype = prototype`: what each instance made after inherits from.
    #[cfg_attr(rust_js, rust_js::link_name = "set prototype")]
    #[allow(unused_variables)]
    pub fn set_prototype(&self, prototype: &Unknown) {
        unreachable!()
    }
}

/// The constructor function of `T`, a struct marked `#[rust_js::class]`:
/// `js::class::<Component>()` is `Component` (ADR 0761). Of a binding's
/// type, the global class TypeScript names it, its `#[rust_js::types]`:
/// `js::class::<AbortController>()` is `AbortController` (ADR 0911). Of
/// any other type, a compile error.
#[cfg_attr(rust_js, rust_js::link_name = "class")]
pub fn class<T>() -> &'static Class {
    unreachable!()
}

/// [`value instanceof constructor`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Operators/instanceof):
/// whether `constructor.prototype` is on the prototype chain of `value`,
/// any value, of a constructor held as a value, as TypeScript's
/// `instanceof` takes `any` on each side (ADR 0878): React's
/// `getPrototypeOf(Uint32Array.prototype).constructor`, or a class's,
/// `js::class::<Component>()`. A test of one class a binding names is
/// that binding's, `#[rust_js::link_name = "instanceof Editor"]`, as the
/// js crate's `data_view::is` is. Of a `constructor` that's no function,
/// JS's `TypeError`, a panic.
#[cfg_attr(rust_js, rust_js::link_name = "instanceof")]
#[allow(unused_variables)]
pub fn instance_of(this: impl IntoUnknown, constructor: impl IntoUnknown) -> bool {
    unreachable!()
}

/// `key in value`: whether an object has a property of that name, its own
/// or its prototype's, as JS's `in` asks, of any value; of what's no
/// object, `undefined` or `null`, a `None`, or a string, JS's `TypeError`,
/// a panic (ADR 0776).
#[cfg_attr(rust_js, rust_js::link_name = "in []")]
#[allow(unused_variables)]
pub fn has<K: PropertyKey + ?Sized>(this: impl IntoUnknown, key: &K) -> bool {
    unreachable!()
}

/// The keys [`for (const key in object)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Statements/for...in)
/// visits, each as the loop reaches it: its own enumerable ones and its
/// prototypes', which [`object::keys`] leaves out, not one a prototype's
/// key of the same name hides, nor one deleted before it's reached (ADR
/// 0719). A `for` loop of it is that loop, `for key in js::keys_in(o)`
/// `for (const key in o)`; any other use of it, a JS iterator of them. Of
/// any value, as JS's loop takes one: of `undefined` or `null`, a `None`,
/// none, as JS visits none, `for (const key in styles)` of an
/// `Option<&Unknown>` (React port gap rd71).
#[cfg_attr(rust_js, rust_js::link_name = "for in")]
#[allow(unused_variables)]
pub fn keys_in(this: impl IntoUnknown) -> Box<dyn core::iter::Iterator<Item = String>> {
    unreachable!()
}

/// What names a property, as `key in value` and `value[key]` take it: text,
/// a number, or a JS value of any shape, whose text JS makes the name, or
/// none, `undefined`, named `"undefined"`.
pub trait PropertyKey {}

impl<T: PropertyKey + ?Sized> PropertyKey for Option<&T> {}

impl PropertyKey for str {}
impl PropertyKey for String {}
impl PropertyKey for Unknown {}
impl PropertyKey for u32 {}
impl PropertyKey for i32 {}
impl PropertyKey for usize {}

/// `value[key] = to`: `to` as JS has it, a string or a number, or an
/// object, set on any value, by a name as [`get`] reads it, text, a
/// number or a JS value (ADR 0967); of `undefined` or `null`, a `None`,
/// JS's `TypeError`, a panic (ADR 0776).
#[cfg_attr(rust_js, rust_js::link_name = "set []")]
#[allow(unused_variables)]
pub fn set<K: PropertyKey + ?Sized, T>(this: impl IntoUnknown, key: &K, to: T) {
    unreachable!()
}

/// A value the browser's [structured clone](https://developer.mozilla.org/docs/Web/API/Web_Workers_API/Structured_clone_algorithm)
/// copies as it is (ADR 0225): what `postMessage`, `pushState` and
/// `structuredClone` take, which a closure or a DOM node isn't, nor
/// `Some(None)`, whose box would arrive as an object. Numbers, strings and
/// `bool`s are, and arrays, tuples and `Vec`s of them, an `Option` of a
/// [`Defined`] one, `Json`, `Dict`, `Unknown`, and what WebIDL marks
/// `[Serializable]`, a `Blob`; `()`, `undefined`. A struct is one as its fields are: unsafe to
/// implement, as its `impl` vouches they are, `unsafe impl StructuredClone for Saved {}`.
pub unsafe trait StructuredClone {}

/// Any value, as TypeScript's `unknown` and `any` take one: what an optional
/// argument of `any` is given by reference, `Option<&dyn IntoUnknown>`, so a
/// `None` needs no type, and a `Some(&value)` is given to JS as it is (ADR
/// 0443).
pub trait IntoUnknown {}
impl<T: ?Sized> IntoUnknown for T {}

/// The arguments a TypeScript rest parameter, `...data: T[]`, is given
/// (ADR 0640): a tuple, its arguments, each of its own type, one a `T`
/// takes ([`Takes`]), `()` of none and `(x,)` of one; or a slice or a
/// `Vec` of them, by reference, spread. A `#[rust_js::variadic]` binding's
/// last parameter: `console::error(("%s is %s", name, kind))` is
/// `console.error("%s is %s", name, kind)`, and `math::max(&values)`
/// `Math.max(...values)`. `T` is `dyn IntoUnknown` of TypeScript's `any`, a
/// union's `dyn Into..` trait, the type an argument is a reference to,
/// `str`, or a number's own type, `f64`.
#[cfg_attr(rust_js, rust_js::rest)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not rest arguments",
    note = "a rest parameter is given a tuple of its arguments, `(a, b)`, `(a,)` of one and `()` of none, or a slice or a `Vec` to spread, `&values`"
)]
pub trait Rest<T: ?Sized> {}

/// What a rest parameter of `T`s takes as one of its arguments, an `A`: any
/// value, of `dyn IntoUnknown`; text, of `str`, a `&str`, a `&String` or a
/// `String`; a number of its own type; and a union's members, or an
/// interface and the ones that extend it, as webapi and node say (ADR
/// 0640). And what [`dict::from_entries`] takes as a key, text.
#[diagnostic::on_unimplemented(message = "a parameter of `{Self}`s takes no `{A}`")]
pub trait Takes<A> {}

impl<A> Takes<A> for dyn IntoUnknown {}
impl<'a> Takes<&'a str> for str {}
impl<'a> Takes<&'a String> for str {}
impl Takes<String> for str {}

/// A number, TypeScript's `number` or `bigint`, of its own type.
macro_rules! takes_itself {
    ($($t:ty),*) => {
        $(impl Takes<$t> for $t {})*
    };
}

takes_itself!(i8, u8, i16, u16, i32, u32, i64, u64, f32);

/// TypeScript's `number` itself, an `f64`: any number JS has as one, each
/// that `f64::from` takes, a `u8` of a `Uint8Array` spread too (ADR 0718).
macro_rules! takes_number {
    ($($t:ty),*) => {
        $(impl Takes<$t> for f64 {})*
    };
}

takes_number!(i8, u8, i16, u16, i32, u32, f32, f64);

/// A slice of them, spread.
impl<'a, T: ?Sized + Takes<A>, A> Rest<T> for &'a [A] {}

/// A `Vec` of them, spread.
impl<'a, T: ?Sized + Takes<A>, A> Rest<T> for &'a Vec<A> {}

/// Rest arguments as a value, spread: what a `fn` whose rest parameter is
/// one is given, so a binding taken as one, `fn(&'static dyn Rest<dyn
/// IntoUnknown>)` of `console::log`, is its JS function itself, `prev =
/// console.log` (ADR 0667).
impl<'a, T: ?Sized> Rest<T> for &'a dyn Rest<T> {}

/// Rest arguments as a value, read: the array of them, as a closure taking
/// them as JS gives them, `|args: &dyn Rest<dyn IntoUnknown>|`, JS's
/// `(...args) =>`, reads it, `for &arg in args.iter()` (ADR 0851). The array
/// itself, of each argument as [`RestItem`] says a `T` reads.
impl<'a, T: ?Sized + RestItem> core::ops::Deref for dyn Rest<T> + 'a {
    type Target = [T::Item];

    fn deref(&self) -> &[T::Item] {
        // Never runs: rust-js compiles this `Deref` to the array itself.
        unreachable!()
    }
}

/// What one of rest arguments of `T`s is, read (ADR 0851): of `dyn
/// IntoUnknown`, any value, an `Option<&Unknown>`; of `str`, text; of a
/// number, its own type. Each is what JS gives, so of any lifetime.
pub trait RestItem {
    type Item;
}

impl RestItem for dyn IntoUnknown {
    type Item = Option<&'static Unknown>;
}
impl RestItem for str {
    type Item = &'static str;
}

/// A number, read as its own type.
macro_rules! rest_number {
    ($($t:ty),*) => {
        $(impl RestItem for $t { type Item = $t; })*
    };
}

rest_number!(i8, u8, i16, u16, i32, u32, i64, u64, f32, f64);

/// A tuple of up to 16, each item one `T` takes.
macro_rules! rest {
    ($(($($a:ident),*))*) => {
        $(
            impl<T: ?Sized $(+ Takes<$a>)*, $($a),*> Rest<T> for ($($a,)*) {}
        )*
    };
}

rest! {
    ()
    (A)
    (A, B)
    (A, B, C)
    (A, B, C, D)
    (A, B, C, D, E)
    (A, B, C, D, E, F)
    (A, B, C, D, E, F, G)
    (A, B, C, D, E, F, G, H)
    (A, B, C, D, E, F, G, H, I)
    (A, B, C, D, E, F, G, H, I, J)
    (A, B, C, D, E, F, G, H, I, J, K)
    (A, B, C, D, E, F, G, H, I, J, K, L)
    (A, B, C, D, E, F, G, H, I, J, K, L, M)
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N)
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O)
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P)
}

/// A value that's never `undefined` nor `null` in JS, as `None` is: an
/// `Option` of one is the value or `undefined`, not a box (ADR 0051), so
/// one a clone copies as it is.
pub unsafe trait Defined {}

/// Numbers, strings and the js crate's own: each copied, and never nullish.
macro_rules! cloned {
    ($($t:ty),* $(,)?) => {
        $(
            unsafe impl StructuredClone for $t {}
            unsafe impl Defined for $t {}
        )*
    };
}

cloned!(bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, char, str, String);
cloned!(Unknown, ArrayBuffer, Json<'_>);
// `()`, `undefined`, which a clone copies as it is, `postMessage(undefined)`:
// never `Defined`, so `Some(())` isn't one, as `None` is it too (ADR 0225).
unsafe impl StructuredClone for () {}
// An object, never nullish, though not one a clone copies, which a function
// in it isn't (ADR 0677).
unsafe impl Defined for JsObject {}

unsafe impl<T: StructuredClone + ?Sized> StructuredClone for &T {}
unsafe impl<T: Defined + ?Sized> Defined for &T {}
unsafe impl<T: StructuredClone + ?Sized> StructuredClone for Box<T> {}
unsafe impl<T: Defined + ?Sized> Defined for Box<T> {}
// An `Rc` is what it points at, where the crate counts none (ADR 0844);
// rust-js refuses one counted where a binding is given it.
unsafe impl<T: Defined + ?Sized> Defined for std::rc::Rc<T> {}
unsafe impl<T: Defined + ?Sized> Defined for std::sync::Arc<T> {}
// A `RefCell` of an object is the object (ADR 0863); rust-js refuses one
// that's a `{ value }` box, of a number, where a binding is given it (ADR
// 0962).
unsafe impl<T: Defined + ?Sized> Defined for std::cell::RefCell<T> {}
unsafe impl<T: StructuredClone> StructuredClone for [T] {}
unsafe impl<T> Defined for [T] {}
unsafe impl<T: StructuredClone, const N: usize> StructuredClone for [T; N] {}
unsafe impl<T, const N: usize> Defined for [T; N] {}
unsafe impl<T: StructuredClone> StructuredClone for Vec<T> {}
unsafe impl<T> Defined for Vec<T> {}
unsafe impl<T: StructuredClone> StructuredClone for Dict<T> {}
unsafe impl<T> Defined for Dict<T> {}
// `Some(x)` is `x` itself only where `x` can't look like `None`.
unsafe impl<T: StructuredClone + Defined> StructuredClone for Option<T> {}

/// A tuple is an array (ADR 0020): copied if its items are, never nullish.
macro_rules! tuples {
    ($(($($t:ident),+))*) => {
        $(
            unsafe impl<$($t: StructuredClone),+> StructuredClone for ($($t,)+) {}
            unsafe impl<$($t),+> Defined for ($($t,)+) {}
        )*
    };
}

tuples! {
    (A)
    (A, B)
    (A, B, C)
    (A, B, C, D)
    (A, B, C, D, E)
    (A, B, C, D, E, F)
}

/// A value whose JS is its JSON, as [`json::stringify_with`] writes it:
/// strings, `bool`s and numbers to 32 bits and floats (a NaN or an infinity
/// is `null`), and arrays, tuples, `Vec`s and `Dict`s of them, `Json`, and
/// an `Option` of one, whose `None` is left out of an object and `null` in
/// an array. Not a 64-bit integer, a `BigInt`, which `JSON.stringify`
/// throws on. A struct is one as its fields are: unsafe to implement, as its
/// `impl` vouches they are, `unsafe impl JsonText for Package {}`.
pub unsafe trait JsonText {}

macro_rules! json_text {
    ($($t:ty),* $(,)?) => {
        $(unsafe impl JsonText for $t {})*
    };
}

json_text!(bool, i8, i16, i32, isize, u8, u16, u32, usize, f32, f64, str, String, Json<'_>);

unsafe impl<T: JsonText + ?Sized> JsonText for &T {}
unsafe impl<T: JsonText + ?Sized> JsonText for Box<T> {}
unsafe impl<T: JsonText> JsonText for [T] {}
unsafe impl<T: JsonText, const N: usize> JsonText for [T; N] {}
unsafe impl<T: JsonText> JsonText for Vec<T> {}
unsafe impl<T: JsonText> JsonText for Dict<T> {}
unsafe impl<T: JsonText + Defined> JsonText for Option<T> {}

/// A plain JS object of `T`s by their names (ADR 0225), as ReScript's `dict`
/// and TypeScript's `Record<string, T>` are: a JSON object, or a
/// dictionary an API takes. [`dict::get`] reads one by its key.
#[cfg_attr(rust_js, rust_js::types = "{ [key: string]: T }")]
pub struct Dict<T>(PhantomData<JsObject>, PhantomData<T>);

/// A JSON value (ADR 0225), as ReScript's `JSON.t` is: each variant's value
/// is the value itself (ADR 0214), and a JSON `null` is `None` of the
/// `Option<Json>` that holds it, an array's item or an object's.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy)]
pub enum Json<'a> {
    String(&'a str),
    Number(f64),
    Bool(bool),
    Array(&'a [Option<Json<'a>>]),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Object(&'a Dict<Option<Json<'a>>>),
}

impl Json<'static> {
    /// [`JSON.parse(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/parse):
    /// the value `text` is, `None` of `null`, or what it threw for text that
    /// isn't JSON (ADR 0035).
    #[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
    #[allow(unused_variables)]
    pub fn parse(text: &str) -> Result<Option<Json<'static>>, &'static JsError> {
        unreachable!()
    }
}

impl Json<'_> {
    /// [`JSON.stringify(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/stringify):
    /// its JSON text.
    #[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
    #[allow(unused_variables)]
    pub fn stringify(value: &Json<'_>) -> String {
        unreachable!()
    }
}

/// A [`Dict`]'s functions.
pub mod dict {
    use super::*;

    /// Its value of `key`, `dict[key]`, as JS reads it: `None` where it has
    /// none, and an inherited key, `toString`, reads what it inherits, as
    /// TypeScript's `Record` and ReScript's `dict` do. Of a
    /// `Dict<Option<_>>`, `Some(None)` of a JSON `null`, and `None` where the
    /// key isn't its own (ADR 0225).
    #[cfg_attr(rust_js, rust_js::link_name = "get []")]
    #[allow(unused_variables)]
    pub fn get<'a, T>(this: &'a Dict<T>, key: &str) -> Option<&'a T> {
        unreachable!()
    }

    /// `{}`: a dictionary of nothing, to fill with [`set`].
    #[cfg_attr(rust_js, rust_js::link_name = "{}")]
    pub fn new<T>() -> &'static Dict<T> {
        unreachable!()
    }

    /// `dict[key] = value`.
    #[cfg_attr(rust_js, rust_js::link_name = "set []")]
    #[allow(unused_variables)]
    pub fn set<T>(this: &Dict<T>, key: &str, value: T) {
        unreachable!()
    }

    /// [`Object.keys(dict)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/keys):
    /// its keys, in order.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.keys")]
    #[allow(unused_variables)]
    pub fn keys<T>(dict: &Dict<T>) -> Vec<String> {
        unreachable!()
    }

    /// [`Object.entries(dict)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/entries):
    /// each key and its value, in order.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.entries")]
    #[allow(unused_variables)]
    pub fn entries<T>(dict: &Dict<T>) -> Vec<(String, T)> {
        unreachable!()
    }

    /// [`Object.values(dict)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/values):
    /// its values, in order.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.values")]
    #[allow(unused_variables)]
    pub fn values<T>(dict: &Dict<T>) -> Vec<T> {
        unreachable!()
    }

    /// [`Object.fromEntries(entries)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/fromEntries):
    /// a dictionary of these keys and values, each key text, a `&str` or a
    /// `String`, as `HashMap::from` takes one: `dict::from_entries(vec![("aria-current",
    /// 0), ..])`. Written out, each key a string, it's the object literal,
    /// `{ "aria-current": 0, .. }` (ADR 0666). It runs none of the
    /// program's code: it reads the pairs it's given, never calls what's in
    /// them (ADR 0967, amended).
    #[cfg_attr(rust_js, rust_js::link_name = "Object.fromEntries")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    #[allow(unused_variables)]
    pub fn from_entries<K, T>(entries: Vec<(K, T)>) -> &'static Dict<T>
    where
        str: Takes<K>,
    {
        unreachable!()
    }
}

/// A JS [`Promise`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise)
/// of a `T`. `.await` on one is JS's `await`; a rejected one throws, like a
/// panic. See ADR 0029.
pub struct Promise<T>(PhantomData<JsObject>, PhantomData<T>);

/// A promise is a handle JS shares: `p.then(f)` leaves `p` as it was, so
/// one of a value that's `Copy` is `Copy` too, read again where it's kept,
/// `info.value.get().unwrap().then_with(..)` of a `Cell<Option<Promise<..>>>`.
/// Of any other value it's consumed, as Rust's: two handlers given the one
/// `Vec` it fulfils with would change one array (ADR 0782).
impl<T: Copy> Clone for Promise<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Copy> Copy for Promise<T> {}

impl<T> Promise<T> {
    /// [`promise.then(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/then):
    /// a promise of what `f` makes of what it fulfils, as ReScript's
    /// `Promise.thenResolve`. `f` gives no promise, which JS would wait for.
    #[cfg_attr(rust_js, rust_js::link_name = "then")]
    #[allow(unused_variables)]
    pub fn then_resolve<U>(self, f: impl FnOnce(T) -> U + 'static) -> Promise<U> {
        unreachable!()
    }
}

impl<T> core::future::Future for Promise<T> {
    type Output = T;

    fn poll(self: core::pin::Pin<&mut Self>, _: &mut core::task::Context<'_>) -> core::task::Poll<T> {
        unreachable!("rust-js compiles `.await` to JS's `await`")
    }
}

/// `promise`, settled either way: its `.await` is `Ok` of what it fulfils
/// with, or `Err` of what it's rejected with, where the `.await` of the
/// promise itself would throw. For a promise of the webapi crate's, as
/// `settle(window.fetch(url)).await` is a network error's `Err`
/// (ADR 0035).
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn settle<T>(this: Promise<T>) -> Promise<Result<T, &'static JsError>> {
    unreachable!()
}

/// A future as the promise it is in JS, where a call of an `async fn` is
/// one already: handed, unawaited, to what takes a promise, as
/// `new ClipboardItem({ "text/plain": promise(fetch_blob()) })`.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
pub fn promise<T>(this: impl core::future::Future<Output = T>) -> Promise<T> {
    unreachable!()
}

/// Run a future without waiting for it, as from an event handler:
/// `spawn(Box::new(async move { .. }))`. A JS promise is already
/// running, so in JS this is the promise itself, left unawaited.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
pub fn spawn(this: Box<dyn core::future::Future<Output = ()>>) {
    unreachable!()
}

/// [`encodeURIComponent`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/encodeURIComponent):
/// `component`, text, a number or a `bool`, for a part of a URL, a query's
/// value say.
#[cfg_attr(rust_js, rust_js::link_name = "encodeURIComponent")]
pub fn encode_uri_component(component: impl IntoStrOrNumberOrBool) -> String {
    unreachable!()
}

/// What a `string | number | boolean` parameter takes, `encodeURIComponent`'s:
/// each as it is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a `string | number | boolean`")]
#[cfg_attr(rust_js, rust_js::types = "string | number | boolean")]
pub trait IntoStrOrNumberOrBool: sealed::Sealed {}
impl IntoStrOrNumberOrBool for &str {}
impl IntoStrOrNumberOrBool for &String {}
impl IntoStrOrNumberOrBool for f64 {}
impl IntoStrOrNumberOrBool for bool {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &String {}
    impl Sealed for f64 {}
    impl Sealed for bool {}
}

/// [`encodeURI`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/encodeURI):
/// `text` for a whole URL, its `/`, `?` and `#` kept.
#[cfg_attr(rust_js, rust_js::link_name = "encodeURI")]
pub fn encode_uri(text: &str) -> String {
    unreachable!()
}

/// [`decodeURIComponent`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/decodeURIComponent):
/// what `encode_uri_component` made, or the `URIError` of what it can't have.
#[cfg_attr(rust_js, rust_js::link_name = "decodeURIComponent")]
pub fn decode_uri_component(text: &str) -> Result<String, &'static JsError> {
    unreachable!()
}

/// [`decodeURI`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/decodeURI):
/// what `encode_uri` made, or the `URIError` of what it can't have.
#[cfg_attr(rust_js, rust_js::link_name = "decodeURI")]
pub fn decode_uri(text: &str) -> Result<String, &'static JsError> {
    unreachable!()
}

/// [`isNaN(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/isNaN):
/// whether `value` is NaN, as `f64::is_nan` says.
#[cfg_attr(rust_js, rust_js::link_name = "isNaN")]
pub fn is_nan(value: f64) -> bool {
    unreachable!()
}

/// [`isFinite(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/isFinite):
/// whether `value` is neither NaN nor infinite, as `f64::is_finite` says.
#[cfg_attr(rust_js, rust_js::link_name = "isFinite")]
pub fn is_finite(value: f64) -> bool {
    unreachable!()
}

/// What `set_timeout` gives, to clear it with: a number in a browser, an
/// object in Node.
pub struct TimeoutId(PhantomData<JsObject>);

/// What `set_interval` gives, to clear it with.
pub struct IntervalId(PhantomData<JsObject>);

// Timers: not the language's own, but every JS runtime's, browsers', workers'
// and Node's, as ReScript's standard library has them (ADR 0102).
/// [`setTimeout(callback, ms)`](https://developer.mozilla.org/docs/Web/API/Window/setTimeout):
/// `callback` once, after at least `ms` milliseconds; of `None`, left
/// out, as soon as it can be, as JS's `setTimeout(callback)`.
#[cfg_attr(rust_js, rust_js::link_name = "setTimeout")]
pub fn set_timeout(callback: Box<dyn FnOnce()>, ms: Option<f64>) -> &'static TimeoutId {
    unreachable!()
}

/// [`clearTimeout(id)`](https://developer.mozilla.org/docs/Web/API/Window/clearTimeout):
/// the timeout's `callback` won't run, if it hasn't; of none, nothing,
/// as TypeScript types it, `number | undefined`.
#[cfg_attr(rust_js, rust_js::link_name = "clearTimeout")]
pub fn clear_timeout(id: Option<&TimeoutId>) {
    unreachable!()
}

/// [`setInterval(callback, ms)`](https://developer.mozilla.org/docs/Web/API/Window/setInterval):
/// `callback` every `ms` milliseconds, until it's cleared; of `None`, left
/// out, as often as it can be, as JS's `setInterval(callback)`.
#[cfg_attr(rust_js, rust_js::link_name = "setInterval")]
pub fn set_interval(callback: Box<dyn FnMut()>, ms: Option<f64>) -> &'static IntervalId {
    unreachable!()
}

/// [`clearInterval(id)`](https://developer.mozilla.org/docs/Web/API/Window/clearInterval):
/// the interval's `callback` doesn't run again; of none, nothing, as
/// TypeScript types it, `number | undefined`.
#[cfg_attr(rust_js, rust_js::link_name = "clearInterval")]
pub fn clear_interval(id: Option<&IntervalId>) {
    unreachable!()
}

/// What `set_immediate` gives, an object of Node's.
pub struct Immediate(PhantomData<JsObject>);

/// [`setImmediate(callback)`](https://nodejs.org/api/timers.html#setimmediatecallback-args):
/// `callback` once, after what's queued now; Node's, Bun's and Deno's, which
/// a browser lacks: `is_defined(set_immediate)` asks (ADR 0597).
#[cfg_attr(rust_js, rust_js::link_name = "setImmediate")]
pub fn set_immediate(callback: Box<dyn FnOnce()>) -> &'static Immediate {
    unreachable!()
}

/// A JS [`RegExp`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp),
/// for what Rust would use the `regex` crate for. `replace` with a string is
/// here; with a closure, and `matchAll`, are bindings a program declares, typed
/// for its pattern: a JS replacer is given the match, then each group, then
/// where it matched, so its Rust type is the pattern's.
pub struct RegExp(PhantomData<JsObject>);

impl RegExp {
    /// [`regexp.test(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/test): whether it matches `text`, from its `last_index` of a `g` or `y` pattern.
    #[cfg_attr(rust_js, rust_js::link_name = "test")]
    pub fn test(&self, text: &str) -> bool {
        unreachable!()
    }

    /// [`regexp.exec(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/exec): its match in `text`, from its `last_index` of a `g` or `y` pattern.
    #[cfg_attr(rust_js, rust_js::link_name = "exec")]
    pub fn exec(&self, text: &str) -> Option<&'static RegExpMatch> {
        unreachable!()
    }

    /// [`regexp.source`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/source): its pattern.
    #[cfg_attr(rust_js, rust_js::link_name = "get source")]
    pub fn source(&self) -> String {
        unreachable!()
    }

    /// [`regexp.flags`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/flags): its flags, `"gm"`.
    #[cfg_attr(rust_js, rust_js::link_name = "get flags")]
    pub fn flags(&self) -> String {
        unreachable!()
    }

    /// [`regexp.lastIndex`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/lastIndex): where its next match starts, of a `g` or `y` pattern.
    #[cfg_attr(rust_js, rust_js::link_name = "get lastIndex")]
    pub fn last_index(&self) -> u32 {
        unreachable!()
    }

    /// [`regexp.lastIndex = index`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/lastIndex): where its next match starts.
    #[cfg_attr(rust_js, rust_js::link_name = "set lastIndex")]
    pub fn set_last_index(&self, index: u32) {
        unreachable!()
    }

    /// [`regexp.global`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/global): whether it has the `g` flag.
    #[cfg_attr(rust_js, rust_js::link_name = "get global")]
    pub fn global(&self) -> bool {
        unreachable!()
    }

    /// [`regexp.ignoreCase`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/ignoreCase): whether it has the `i` flag.
    #[cfg_attr(rust_js, rust_js::link_name = "get ignoreCase")]
    pub fn ignore_case(&self) -> bool {
        unreachable!()
    }

    /// [`regexp.multiline`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/multiline): whether it has the `m` flag.
    #[cfg_attr(rust_js, rust_js::link_name = "get multiline")]
    pub fn multiline(&self) -> bool {
        unreachable!()
    }

    /// [`regexp.dotAll`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/dotAll): whether it has the `s` flag.
    #[cfg_attr(rust_js, rust_js::link_name = "get dotAll")]
    pub fn dot_all(&self) -> bool {
        unreachable!()
    }

    /// [`regexp.unicode`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/unicode): whether it has the `u` flag.
    #[cfg_attr(rust_js, rust_js::link_name = "get unicode")]
    pub fn unicode(&self) -> bool {
        unreachable!()
    }

    /// [`regexp.unicodeSets`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/unicodeSets): whether it has the `v` flag.
    #[cfg_attr(rust_js, rust_js::link_name = "get unicodeSets")]
    pub fn unicode_sets(&self) -> bool {
        unreachable!()
    }

    /// [`regexp.sticky`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/sticky): whether it has the `y` flag.
    #[cfg_attr(rust_js, rust_js::link_name = "get sticky")]
    pub fn sticky(&self) -> bool {
        unreachable!()
    }

    /// [`regexp.hasIndices`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/hasIndices): whether it has the `d` flag.
    #[cfg_attr(rust_js, rust_js::link_name = "get hasIndices")]
    pub fn has_indices(&self) -> bool {
        unreachable!()
    }
}

/// What [`exec`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/exec) finds: the match, its groups, where it is.
pub struct RegExpMatch(PhantomData<JsObject>);

impl RegExpMatch {
    /// `m[0]`: what matched, ReScript's `fullMatch`.
    #[cfg_attr(rust_js, rust_js::link_name = "get 0")]
    pub fn full_match(&self) -> String {
        unreachable!()
    }

    /// `m[i]`: the match, 0, or its group `i`, `None` of one that matched nothing.
    #[cfg_attr(rust_js, rust_js::link_name = "get []")]
    pub fn get(&self, i: u32) -> Option<String> {
        unreachable!()
    }

    /// `m.length`: the match and its groups, how many.
    #[cfg_attr(rust_js, rust_js::link_name = "get length")]
    pub fn length(&self) -> u32 {
        unreachable!()
    }

    /// `m.index`: where in the text it is.
    #[cfg_attr(rust_js, rust_js::link_name = "get index")]
    pub fn index(&self) -> u32 {
        unreachable!()
    }

    /// `m.input`: the text it's in.
    #[cfg_attr(rust_js, rust_js::link_name = "get input")]
    pub fn input(&self) -> String {
        unreachable!()
    }

    /// `m.groups`: its named groups, each by its name.
    #[cfg_attr(rust_js, rust_js::link_name = "get groups")]
    pub fn groups(&self) -> Option<&'static Dict<String>> {
        unreachable!()
    }
}


pub mod reg_exp {
    use super::*;

    /// `new RegExp(pattern, flags)`: flags like `"gm"`.
    /// Of a pattern and flags written as they are, it's their literal,
    /// `/%s/g`, as JS writes one.
    #[cfg_attr(rust_js, rust_js::link_name = "new RegExp")]
    pub fn new(pattern: &str, flags: &str) -> &'static RegExp {
        unreachable!()
    }

    /// [`RegExp.escape(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/RegExp/escape):
    /// `text` as a pattern that matches it as it is, as ReScript's Stdlib has it (ES2025).
    #[cfg_attr(rust_js, rust_js::link_name = "RegExp.escape")]
    pub fn escape(text: &str) -> String {
        unreachable!()
    }

    /// [`text.replace(pattern, with)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/replace):
    /// the first match, or with the `g` flag each, replaced with `with`:
    /// text, in which `$1` is the first group and `$&` the match, or a
    /// closure of the match and its groups, each `None` where it matched
    /// nothing, giving the text, as TypeScript's overloads take either
    /// (ADR 0310).
    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    #[allow(unused_variables)]
    pub fn replace<A>(this: &str, pattern: &RegExp, with: impl Replacement<A>) -> String {
        unreachable!()
    }

    /// What [`replace`] replaces a match with: text, or a closure of the
    /// match and up to 9 of its groups, giving text or any value, which JS
    /// makes text. `A` tells the closure's arity apart.
    pub trait Replacement<A> {}

    impl Replacement<()> for &str {}
    impl Replacement<()> for &String {}

    macro_rules! replacement {
        ($($group:ident)*) => {
            impl<F: FnMut(&str $(, replacement!(@group $group))*) -> R, R: ToText> Replacement<(R, $(replacement!(@marker $group),)*)> for F {}
        };
        (@group $group:ident) => { Option<&str> };
        (@marker $group:ident) => { () };
    }

    replacement!();
    replacement!(g1);
    replacement!(g1 g2);
    replacement!(g1 g2 g3);
    replacement!(g1 g2 g3 g4);
    replacement!(g1 g2 g3 g4 g5);
    replacement!(g1 g2 g3 g4 g5 g6);
    replacement!(g1 g2 g3 g4 g5 g6 g7);
    replacement!(g1 g2 g3 g4 g5 g6 g7 g8);
    replacement!(g1 g2 g3 g4 g5 g6 g7 g8 g9);
}

/// A JS string's methods, as ReScript's standard library names them, for
/// code that counts as JS does: by UTF-16 code units, where Rust's `str`
/// counts bytes, so `"😀".len()` is 4 and its JS `length` 2. A negative
/// index of `slice` counts from the end. Where a string is searched, and
/// what `slice` takes of what's found, is a JS number, `f64`, as TypeScript
/// has it: one past it is `+ 1`. A count and what a loop indexes by is a
/// `u32`.
pub mod string {
    /// [`text.length`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/length):
    /// its UTF-16 code units.
    #[cfg_attr(rust_js, rust_js::link_name = "get length")]
    pub fn length(this: &str) -> u32 {
        unreachable!()
    }

    /// [`String.fromCharCode(..codes)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/fromCharCode):
    /// the text of these UTF-16 code units, each a number, TypeScript's
    /// `number`, as JS makes one of it: its low 16 bits. A rest parameter,
    /// given a tuple, `from_char_code((72, 105))`, or a slice, a `Vec` or a
    /// typed array spread, `String.fromCharCode(...bytes)` (ADR 0718).
    #[cfg_attr(rust_js, rust_js::link_name = "String.fromCharCode")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn from_char_code(codes: impl super::Rest<f64>) -> String {
        unreachable!()
    }

    /// [`text.charAt(index)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/charAt):
    /// its UTF-16 code unit at `index`, half of an emoji, or `""` past its end.
    #[cfg_attr(rust_js, rust_js::link_name = "charAt")]
    pub fn char_at(this: &str, index: u32) -> String {
        unreachable!()
    }

    /// `text[index]`: its UTF-16 code unit at `index`, any number, as text,
    /// half of an emoji; `None` where JS reads `undefined`, past its end,
    /// before its start or of a fraction, as React's FallbackCompositionState
    /// compares `endValue[endLength - end]` (ADR 0994).
    #[cfg_attr(rust_js, rust_js::link_name = "get []")]
    pub fn get(this: &str, index: f64) -> Option<&'static str> {
        unreachable!()
    }

    /// [`text.charCodeAt(index)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/charCodeAt):
    /// its UTF-16 code unit at `index`, a number, `NaN` past its end.
    #[cfg_attr(rust_js, rust_js::link_name = "charCodeAt")]
    pub fn char_code_at(this: &str, index: u32) -> f64 {
        unreachable!()
    }

    /// [`text.slice(start, end)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/slice):
    /// from `start` to before `end`, each counted from the end if negative;
    /// to the end where there's no `end`.
    #[cfg_attr(rust_js, rust_js::link_name = "slice")]
    pub fn slice(this: &str, start: f64, end: Option<f64>) -> String {
        unreachable!()
    }

    /// [`text.substring(start, end)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/substring):
    /// from `start` to before `end`, the two swapped if `start` is after it;
    /// to the end where there's no `end`.
    #[cfg_attr(rust_js, rust_js::link_name = "substring")]
    pub fn substring(this: &str, start: u32, end: Option<u32>) -> String {
        unreachable!()
    }

    /// [`text.indexOf(search)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/indexOf):
    /// where `search` first is, from `from` on where it's given, or `-1`.
    #[cfg_attr(rust_js, rust_js::link_name = "indexOf")]
    #[cfg_attr(rust_js, rust_js::whole = "-1..=4294967294")]
    pub fn index_of(this: &str, search: &str, from: Option<f64>) -> f64 {
        unreachable!()
    }

    /// [`text.lastIndexOf(search)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/lastIndexOf):
    /// where `search` last is, or `-1`.
    #[cfg_attr(rust_js, rust_js::link_name = "lastIndexOf")]
    #[cfg_attr(rust_js, rust_js::whole = "-1..=4294967294")]
    pub fn last_index_of(this: &str, search: &str) -> f64 {
        unreachable!()
    }

    /// [`text.replace(pattern, replacement)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/replace):
    /// its first `pattern` replaced, where Rust's `str::replace` replaces each;
    /// `$&` in `replacement` is the match.
    #[cfg_attr(rust_js, rust_js::link_name = "replace")]
    pub fn replace(this: &str, pattern: &str, replacement: &str) -> String {
        unreachable!()
    }

    /// [`text.split(separator)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/split):
    /// the text between each `separator`, an empty one each UTF-16 code unit.
    #[cfg_attr(rust_js, rust_js::link_name = "split")]
    pub fn split(this: &str, separator: &str) -> Vec<String> {
        unreachable!()
    }

    /// [`text.split(pattern)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/split):
    /// the text between each match of `pattern`, one of no groups: a group's
    /// match is put between, which a binding a program declares types.
    #[cfg_attr(rust_js, rust_js::link_name = "split")]
    pub fn split_by_reg_exp(this: &str, pattern: &super::RegExp) -> Vec<String> {
        unreachable!()
    }

    /// [`text.trim()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/String/trim):
    /// without JS's white space and line ends at either end, which aren't
    /// Rust's `trim`'s quite: JS's has U+FEFF, Rust's U+0085.
    #[cfg_attr(rust_js, rust_js::link_name = "trim")]
    pub fn trim(this: &str) -> String {
        unreachable!()
    }

    /// `text.trimStart()`: without JS's white space at its start.
    #[cfg_attr(rust_js, rust_js::link_name = "trimStart")]
    pub fn trim_start(this: &str) -> String {
        unreachable!()
    }

    /// `text.trimEnd()`: without JS's white space at its end.
    #[cfg_attr(rust_js, rust_js::link_name = "trimEnd")]
    pub fn trim_end(this: &str) -> String {
        unreachable!()
    }

    /// `text.localeCompare(that)`: how `text` sorts beside `that` in the
    /// default locale, as an Intl collator gives it, -1, 0 or 1.
    #[cfg_attr(rust_js, rust_js::link_name = "localeCompare")]
    pub fn locale_compare(this: &str, that: &str) -> core::cmp::Ordering {
        unreachable!()
    }
}

/// A JS number's methods, for where Rust's own `format!` isn't what's wanted.
pub mod number {
    /// [`x.toFixed(digits)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Number/toFixed):
    /// `x` with `digits` decimals, as JS rounds, a tie away from zero:
    /// `2.5` is `"3"`, where `format!("{:.0}", 2.5)` is `"2"`, to even, as
    /// Rust rounds it. Between ties they agree. `-0` is `"0"`, `1e21` and up
    /// is `String(x)`, `"1e+21"`, and `Infinity` is `"Infinity"`, where
    /// `format!`'s is `"inf"`. `digits` is 0 to 100: more throws a `RangeError`.
    #[cfg_attr(rust_js, rust_js::link_name = "toFixed")]
    pub fn to_fixed(this: f64, digits: u32) -> String {
        unreachable!()
    }

    /// [`x.toString(radix)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Number/toString):
    /// `x` as JS writes it, in base `radix`, 2 to 36, or ten where it's
    /// `None`, TypeScript's `radix?: number`: `1e21` is `"1e+21"` and
    /// `Infinity` `"Infinity"`, where `f64::to_string`'s are
    /// `"1000000000000000000000"` and `"inf"` (ADR 0310); `to_string(35.0,
    /// Some(36))` is `"z"`, as React writes an index as a key (ADR 0860).
    /// Another radix throws a `RangeError`.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(this: f64, radix: Option<u32>) -> String {
        unreachable!()
    }
}

/// JS's [`Math`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Math),
/// where its answer isn't the `f64` method's: a program that wants JS's
/// asks for it here. The rest, `x.sqrt()` say, are `Math`'s already
/// (ADR 0283).
pub mod math {
    use super::Rest;

    /// [`Math.round(x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Math/round):
    /// the nearest integer, a half up, `-2.5` to `-2`, where `x.round()`
    /// takes it away from zero, to `-3`.
    #[cfg_attr(rust_js, rust_js::link_name = "Math.round")]
    #[cfg_attr(rust_js, rust_js::runs_only_given)]
    pub fn round(x: f64) -> f64 {
        unreachable!()
    }

    /// [`Math.max(..values)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Math/max):
    /// the largest, `NaN` if any is, and `0` above `-0`, where `a.max(b)`
    /// is the other number of a `NaN`. Of none, `-Infinity`.
    #[cfg_attr(rust_js, rust_js::link_name = "Math.max")]
    #[cfg_attr(rust_js, rust_js::runs_only_given)]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn max(values: impl Rest<f64>) -> f64 {
        unreachable!()
    }

    /// [`Math.min(..values)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Math/min):
    /// the smallest, `NaN` if any is, and `-0` below `0`, where `a.min(b)`
    /// is the other number of a `NaN`. Of none, `Infinity`.
    #[cfg_attr(rust_js, rust_js::link_name = "Math.min")]
    #[cfg_attr(rust_js, rust_js::runs_only_given)]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn min(values: impl Rest<f64>) -> f64 {
        unreachable!()
    }

    /// [`Math.sign(x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Math/sign):
    /// `1`, `-1`, or `x` itself of a zero or `NaN`, where `x.signum()` is
    /// `1` of `0`.
    #[cfg_attr(rust_js, rust_js::link_name = "Math.sign")]
    #[cfg_attr(rust_js, rust_js::runs_only_given)]
    pub fn sign(x: f64) -> f64 {
        unreachable!()
    }

    /// [`Math.random()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Math/random):
    /// a number from `0` up to `1`, not one to make a secret of.
    #[cfg_attr(rust_js, rust_js::link_name = "Math.random")]
    #[cfg_attr(rust_js, rust_js::runs_only_given)]
    pub fn random() -> f64 {
        unreachable!()
    }
}

/// JS's [`JSON`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON).
/// A Rust value's JSON is serde's (ADR 0077): `JSON.stringify` of one, as
/// rust-js has it in JS, isn't, since `None` is `undefined` and an `i64` a
/// `BigInt`, which it throws on. So it's typed for what it's exact for.
pub mod json {
    use super::*;

    /// [`JSON.parse(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/parse):
    /// the value `text` is, `None` of `null`, or what it threw for JSON
    /// that isn't (ADR 0035).
    #[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
    pub fn parse(text: &str) -> Result<Option<&'static Unknown>, &'static JsError> {
        unreachable!()
    }

    /// [`JSON.parse(text, reviver)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/parse#the_reviver_parameter):
    /// the value `text` is, each part of it given to `reviver` with its
    /// key, innermost first, and made what that gives; a property it gives
    /// `None` of is left out. Of JSON a program made itself: JSON that
    /// isn't throws, a panic.
    #[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
    pub fn parse_with(
        text: &str,
        reviver: Box<dyn Fn(&str, Option<&'static Unknown>) -> Option<&'static Unknown>>,
    ) -> Option<&'static Unknown> {
        unreachable!()
    }

    /// [`JSON.stringify(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/stringify):
    /// `text`'s JSON, in quotes, with its `"`, `\` and control characters
    /// escaped, which is a JS string literal too.
    #[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
    pub fn stringify(text: &str) -> String {
        unreachable!()
    }

    /// [`JSON.stringify(value, replacer, space)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/JSON/stringify):
    /// `value`'s JSON, each level indented by `space` spaces, up to 10, or
    /// on one line of 0. An object has only the properties `replacer` names,
    /// if it names any.
    ///
    /// Of any value, an [`Unknown`], what may throw, of a cycle or a
    /// `BigInt`, or be `undefined`, of a function (ADR 0310).
    #[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
    #[cfg_attr(rust_js, rust_js::nullable(replacer))]
    #[allow(unused_variables)]
    pub fn stringify_with<T: Stringify + ?Sized>(value: &T, replacer: Option<&[&str]>, space: u32) -> T::Stringified {
        unreachable!()
    }

    /// What [`stringify_with`] gives of a value: the text of one whose JSON
    /// is exact, a [`JsonText`]; of any value, what it may throw or leave
    /// `undefined`.
    pub trait Stringify {
        type Stringified;
    }

    impl<T: JsonText + ?Sized> Stringify for T {
        type Stringified = String;
    }

    impl Stringify for Unknown {
        type Stringified = Result<Option<String>, &'static JsError>;
    }

    impl Stringify for Option<&Unknown> {
        type Stringified = Result<Option<String>, &'static JsError>;
    }
}

/// Whatever a JS function threw, or a promise rejected with: usually an
/// [`Error`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error).
/// An `extern` function that returns `Result<T, &JsError>` catches it (ADR 0035).
pub struct JsError(PhantomData<JsObject>);

/// What JS shows of it, `String(error)`, `SyntaxError: ..`: an `unwrap`'s
/// panic says it.
impl core::fmt::Debug for JsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&js_error::to_string(self))
    }
}

impl JsError {
    /// `e instanceof Error`: what was thrown is an
    /// [`Error`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error),
    /// which has a `message`. A promise may be rejected with anything.
    #[cfg_attr(rust_js, rust_js::link_name = "instanceof Error")]
    pub fn is_error(&self) -> bool {
        unreachable!()
    }

    /// [`e.message`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error/message):
    /// an `Error`'s message, without its name, as `is_error()` says it is one.
    #[cfg_attr(rust_js, rust_js::link_name = "get message")]
    pub fn message(&self) -> String {
        unreachable!()
    }

    /// [`e.name`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error/name):
    /// its kind, `"TypeError"`, of an `Error`.
    #[cfg_attr(rust_js, rust_js::link_name = "get name")]
    pub fn name(&self) -> String {
        unreachable!()
    }

    /// What was thrown, as a value of any shape, `None` of `undefined` and
    /// `null`: the value itself, which a parameter of any value takes, as
    /// `node.v = error` stores it.
    #[cfg_attr(rust_js, rust_js::link_name = "this")]
    pub fn as_unknown(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    /// [`e.cause`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error/cause):
    /// what caused it, of any shape, `None` of none.
    #[cfg_attr(rust_js, rust_js::link_name = "get cause")]
    pub fn cause(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    /// [`e.stack`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error/stack):
    /// where it was made, as the runtime writes it, `None` where it has none.
    #[cfg_attr(rust_js, rust_js::link_name = "get stack")]
    pub fn stack(&self) -> Option<String> {
        unreachable!()
    }
}

pub mod js_error {
    use super::*;

    /// `String(e)`: an `Error`'s name and message, or any value as text.
    #[cfg_attr(rust_js, rust_js::link_name = "String")]
    pub fn to_string(error: &JsError) -> String {
        unreachable!()
    }
}

/// An [`Error`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error)
/// object, TypeScript's `Error`: what `error::new` makes, and what a
/// function given one takes, as React's `createCapturedValueFromError(value:
/// Error)` does. It's what may be thrown too, a `&JsError` by `Deref`, which
/// reads its `message`, `name` and `stack`.
#[cfg_attr(rust_js, rust_js::types = "Error")]
pub struct Error(PhantomData<JsObject>);

/// What JS shows of it, as of what's thrown: `Error: ..`.
impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&js_error::to_string(self))
    }
}

impl core::ops::Deref for Error {
    type Target = JsError;

    fn deref(&self) -> &JsError {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const JsError) }
    }
}

pub mod error {
    use super::*;

    /// [`new Error(message)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Error/Error):
    /// an `Error` of `message`, to reject a promise with, or to throw.
    #[cfg_attr(rust_js, rust_js::link_name = "new Error")]
    pub fn new(message: &str) -> &'static Error {
        unreachable!()
    }
}

/// An [`AggregateError`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/AggregateError):
/// several errors as one, what `Promise.any` rejects with when every
/// promise rejects. It's an `Error`, a `&Error` by `Deref`.
#[cfg_attr(rust_js, rust_js::types = "AggregateError")]
pub struct AggregateError(PhantomData<JsObject>);

impl core::ops::Deref for AggregateError {
    type Target = Error;

    fn deref(&self) -> &Error {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const Error) }
    }
}

impl AggregateError {
    /// [`e.errors`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/AggregateError/errors):
    /// the errors it holds, each of any value, TypeScript's `any[]`.
    #[cfg_attr(rust_js, rust_js::link_name = "get errors")]
    pub fn errors(&self) -> Vec<Option<&'static Unknown>> {
        unreachable!()
    }
}

pub mod aggregate_error {
    use super::*;

    /// [`new AggregateError(errors, message)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/AggregateError/AggregateError):
    /// an `AggregateError` of `errors`, each of any value, TypeScript's
    /// `Iterable<any>`, and of `message`, or none. It runs none of the
    /// crate's code: it reads the array, not what's in it.
    #[cfg_attr(rust_js, rust_js::link_name = "new AggregateError")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn new(errors: &[Option<&Unknown>], message: Option<&str>) -> &'static AggregateError {
        unreachable!()
    }
}

/// JS's [`Object`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object)
/// functions, for a JS object an API takes or gives.
pub mod object {
    use super::*;

    /// [`Object.keys(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/keys):
    /// the names of an object's own properties, in order, each one
    /// [`get`] reads.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.keys")]
    pub fn keys(value: &Unknown) -> Vec<String> {
        unreachable!()
    }

    /// [`Object.prototype.toString.call(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/toString):
    /// what JS names any value's kind, `"[object Array]"`; what it throws
    /// of a value whose `Symbol.toStringTag` throws (ADR 0310).
    #[cfg_attr(rust_js, rust_js::link_name = "Object.prototype.toString.call")]
    pub fn to_string(value: Option<&Unknown>) -> Result<String, &'static JsError> {
        unreachable!()
    }

    /// [`Object.fromEntries(entries)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/fromEntries):
    /// a JS object of these keys and values, as an API that takes a
    /// dictionary of them wants: `{ "aria-label": "Rust source" }`.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.fromEntries")]
    #[allow(unused_variables)]
    pub fn from_entries<T>(entries: Vec<(String, T)>) -> &'static JsObject {
        unreachable!()
    }

    /// [`{ ...value }`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Operators/Spread_syntax#spread_in_object_literals):
    /// a new object of `value`'s own enumerable properties, each defined on
    /// it as JS's spread defines it, a `__proto__` key too; `{}` of `None`,
    /// `undefined` or `null` (ADR 0967). Not `assign(object, value)`, which
    /// sets each, so a `__proto__` key sets the new object's prototype.
    /// Of a [`Dict`], a `Dict` of its `T`s; of a tuple of them, `{ ...a,
    /// ...b }`, each spread in order, a later key's value the one kept, as
    /// a `Dict` written out after another is: `spread((EVENT_INTERFACE,
    /// dict::from_entries(vec![("view", 0)])))`'s `{ ...EventInterface,
    /// view: 0 }` (ADR 0967, amended).
    #[cfg_attr(rust_js, rust_js::link_name = "{...}")]
    #[allow(unused_variables)]
    pub fn spread<S: Spread>(values: S) -> &'static S::Object {
        unreachable!()
    }

    /// What [`spread`] spreads, and what it makes of it: any value, `None`
    /// of `undefined` or `null`, a `JsObject`; a [`Dict`], a `Dict` of its
    /// `T`s; or a tuple of two or three of one kind, spread in order.
    pub trait Spread {
        type Object: ?Sized + 'static;
    }

    impl Spread for Option<&Unknown> {
        type Object = JsObject;
    }

    impl<T: 'static> Spread for &Dict<T> {
        type Object = Dict<T>;
    }

    impl<A: Spread, B: Spread<Object = A::Object>> Spread for (A, B) {
        type Object = A::Object;
    }

    impl<A: Spread, B: Spread<Object = A::Object>, C: Spread<Object = A::Object>> Spread for (A, B, C) {
        type Object = A::Object;
    }

    /// [`Object.assign(target, ...sources)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/assign):
    /// each source's own enumerable properties set on `target`, in order,
    /// a `None` source, `null` or `undefined`, skipped; `target`.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.assign")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn assign<'a>(target: &'a Unknown, sources: impl Rest<dyn IntoUnknown>) -> &'a Unknown {
        unreachable!()
    }

    /// [`Object.getPrototypeOf(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/getPrototypeOf):
    /// the object `value` inherits from, or `None`, `null`, of one that
    /// inherits from none, `Object.create(null)`'s. Of any value, as
    /// TypeScript's `any`, `js::get(..)`'s too; JS throws a `TypeError` of
    /// `None`, `undefined` or `null` (ADR 0666, amended).
    #[cfg_attr(rust_js, rust_js::link_name = "Object.getPrototypeOf")]
    pub fn get_prototype_of(value: Option<&Unknown>) -> Option<&'static Unknown> {
        unreachable!()
    }

    /// [`Object.prototype.hasOwnProperty`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/hasOwnProperty),
    /// the function: [`has_own_property`]`.call(object, key)`, whether
    /// `object` has a property of that name of its own, which no
    /// `hasOwnProperty` of its own, nor none, changes (ADR 0615).
    pub struct HasOwnProperty(PhantomData<JsObject>);

    impl HasOwnProperty {
        /// `hasOwnProperty.call(object, key)`.
        #[cfg_attr(rust_js, rust_js::link_name = "call")]
        pub fn call<K: PropertyKey + ?Sized>(&self, object: &Unknown, key: &K) -> bool {
            unreachable!()
        }
    }

    unsafe extern "Rust" {
        /// `Object.prototype.hasOwnProperty`, a JS function of its own, to
        /// [`HasOwnProperty::call`] with an object.
        #[link_name = "Object.prototype.hasOwnProperty"]
        pub safe static has_own_property: &'static HasOwnProperty;
    }

    /// [`Object.is(a, b)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/is):
    /// the same object, or the same value, where `NaN` is itself and `0`
    /// isn't `-0`. Of two JS objects of one type, `std::ptr::eq` says as
    /// much (ADR 0285); this, of two Rust types too, since a JS object may
    /// be seen as either: a message's sender, an object, and a frame's `Window`.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.is")]
    #[allow(unused_variables)]
    pub fn is<A: ?Sized, B: ?Sized>(a: &A, b: &B) -> bool {
        unreachable!()
    }

    unsafe extern "Rust" {
        /// [`Object.prototype`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/prototype):
        /// what a plain object inherits from, as a value, `object ==
        /// object::prototype` being `object === Object.prototype`.
        #[link_name = "Object.prototype"]
        pub safe static prototype: &'static Unknown;
    }

    /// [`Object.defineProperty(o, key, attributes)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/defineProperty):
    /// `o`, its property now as `attributes` say; what JS throws of one it
    /// can't define, `Reflect.defineProperty`'s `false`, as a panic.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.defineProperty")]
    pub fn define_property<'a, T: ?Sized, K: PropertyKey + ?Sized>(o: &'a T, key: &K, attributes: &PropertyDescriptor<'_>) -> &'a T {
        unreachable!()
    }

    /// [`Object.defineProperties(o, properties)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/defineProperties):
    /// `o`, each property as its descriptor says, `{ log: props, .. }`.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.defineProperties")]
    pub fn define_properties<'a, T: ?Sized>(o: &'a T, properties: &dyn PropertyDescriptorMap) -> &'a T {
        unreachable!()
    }

    /// [`Object.getOwnPropertyDescriptor(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/getOwnPropertyDescriptor):
    /// what an own property of `o` is, `None` where it has none. Of any
    /// value, as TypeScript's `any`, `js::get(node, "constructor")`'s too;
    /// JS throws a `TypeError` of `None` (ADR 0666, amended).
    #[cfg_attr(rust_js, rust_js::link_name = "Object.getOwnPropertyDescriptor")]
    pub fn get_own_property_descriptor<K: PropertyKey + ?Sized>(o: Option<&Unknown>, key: &K) -> Option<PropertyDescriptor<'static>> {
        unreachable!()
    }

    /// [`Object.getOwnPropertyDescriptors(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/getOwnPropertyDescriptors):
    /// each own property's descriptor, by its name.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.getOwnPropertyDescriptors")]
    pub fn get_own_property_descriptors(o: &Unknown) -> &'static Dict<PropertyDescriptor<'static>> {
        unreachable!()
    }

    /// [`Object.getOwnPropertyNames(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/getOwnPropertyNames):
    /// the names of `o`'s own properties, enumerable or not, which
    /// [`keys`] gives of the enumerable ones; not its symbols'. Of any
    /// value, as TypeScript's `any`; JS throws a `TypeError` of `None`
    /// (ADR 0666, amended).
    #[cfg_attr(rust_js, rust_js::link_name = "Object.getOwnPropertyNames")]
    pub fn get_own_property_names(o: Option<&Unknown>) -> Vec<String> {
        unreachable!()
    }

    /// [`Object.getOwnPropertySymbols(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/getOwnPropertySymbols):
    /// the symbols `o`'s own properties are keyed by.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.getOwnPropertySymbols")]
    pub fn get_own_property_symbols(o: &Unknown) -> Vec<&'static Symbol> {
        unreachable!()
    }

    /// [`Object.create(prototype, properties)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/create):
    /// an object inheriting from `prototype`, from none of `None`, `null`,
    /// of these properties, where they're given.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.create")]
    #[cfg_attr(rust_js, rust_js::nullable(o))]
    pub fn create(o: Option<&Unknown>, properties: Option<&dyn PropertyDescriptorMap>) -> &'static Unknown {
        unreachable!()
    }

    /// [`Object.setPrototypeOf(o, prototype)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/setPrototypeOf):
    /// `o`, now inheriting from `prototype`, from none of `None`, `null`.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.setPrototypeOf")]
    #[cfg_attr(rust_js, rust_js::nullable(proto))]
    pub fn set_prototype_of<'a>(o: &'a Unknown, proto: Option<&Unknown>) -> &'a Unknown {
        unreachable!()
    }

    /// [`Object.freeze(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/freeze):
    /// `o`, none of its properties changed, added or removed from now on.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.freeze")]
    pub fn freeze<T: ?Sized>(o: &T) -> &T {
        unreachable!()
    }

    /// [`Object.isFrozen(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/isFrozen).
    #[cfg_attr(rust_js, rust_js::link_name = "Object.isFrozen")]
    pub fn is_frozen(o: &Unknown) -> bool {
        unreachable!()
    }

    /// [`Object.seal(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/seal):
    /// `o`, none of its properties added or removed from now on.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.seal")]
    pub fn seal<T: ?Sized>(o: &T) -> &T {
        unreachable!()
    }

    /// [`Object.isSealed(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/isSealed).
    #[cfg_attr(rust_js, rust_js::link_name = "Object.isSealed")]
    pub fn is_sealed(o: &Unknown) -> bool {
        unreachable!()
    }

    /// [`Object.preventExtensions(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/preventExtensions):
    /// `o`, no property added to it from now on.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.preventExtensions")]
    pub fn prevent_extensions<T: ?Sized>(o: &T) -> &T {
        unreachable!()
    }

    /// [`Object.isExtensible(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/isExtensible):
    /// whether a property may be added to it.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.isExtensible")]
    pub fn is_extensible(o: &Unknown) -> bool {
        unreachable!()
    }

    /// [`Object.hasOwn(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/hasOwn):
    /// whether `o` has a property of that name of its own, not its
    /// prototype's, which `key in o`, [`has`], asks of either.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.hasOwn")]
    pub fn has_own<K: PropertyKey + ?Sized>(o: &Unknown, key: &K) -> bool {
        unreachable!()
    }

    /// [`Object.groupBy(items, keySelector)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/groupBy):
    /// `items` by the key `key_selector` gives each, with its index, in
    /// order; a key none has is `None`.
    #[cfg_attr(rust_js, rust_js::link_name = "Object.groupBy")]
    pub fn group_by<K: PropertyKey, T>(items: Vec<T>, key_selector: impl FnMut(&T, u32) -> K + 'static) -> &'static Dict<Vec<T>> {
        unreachable!()
    }
}

// The crate's objects, which JS holds weakly; a symbol of its own too.
unsafe impl WeakKey for JsObject {}
unsafe impl WeakKey for RegExp {}
unsafe impl WeakKey for JsError {}
unsafe impl WeakKey for Error {}
unsafe impl WeakKey for AggregateError {}
unsafe impl WeakKey for Date {}
unsafe impl WeakKey for Symbol {}
unsafe impl WeakKey for ArrayBuffer {}
unsafe impl WeakKey for SharedArrayBuffer {}
unsafe impl<B> WeakKey for ArrayBufferView<B> {}
unsafe impl<B> WeakKey for DataView<B> {}
unsafe impl<B> WeakKey for Int8Array<B> {}
unsafe impl<B> WeakKey for Uint8Array<B> {}
unsafe impl<B> WeakKey for Uint8ClampedArray<B> {}
unsafe impl<B> WeakKey for Int16Array<B> {}
unsafe impl<B> WeakKey for Uint16Array<B> {}
unsafe impl<B> WeakKey for Int32Array<B> {}
unsafe impl<B> WeakKey for Uint32Array<B> {}
unsafe impl<B> WeakKey for Float32Array<B> {}
unsafe impl<B> WeakKey for Float64Array<B> {}
unsafe impl<B> WeakKey for BigInt64Array<B> {}
unsafe impl<B> WeakKey for BigUint64Array<B> {}
unsafe impl<T> WeakKey for Dict<T> {}
unsafe impl<T> WeakKey for Promise<T> {}

// The crate's objects, never `undefined` nor `null`: each a value of any
// shape, `js::unknown(js::set::new(Some(&[wakeable])))`, `new
// Set([wakeable])` where React stores one in an `any` field. Not a
// `JsError`, which is whatever was thrown, `undefined` too.
unsafe impl Defined for RegExp {}
unsafe impl Defined for Error {}
unsafe impl Defined for AggregateError {}
unsafe impl<T> Defined for Promise<T> {}
unsafe impl<T> Defined for Iterator<T> {}
unsafe impl<K: ?Sized, V> Defined for Map<K, V> {}
unsafe impl<T: ?Sized> Defined for Set<T> {}
unsafe impl<K: WeakKey + ?Sized, V> Defined for WeakMap<K, V> {}
unsafe impl<T: WeakKey + ?Sized> Defined for WeakSet<T> {}
unsafe impl<T: WeakKey + ?Sized + 'static> Defined for WeakRef<T> {}
unsafe impl<T> Defined for FinalizationRegistry<T> {}
