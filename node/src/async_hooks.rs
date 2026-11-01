//! [`async_hooks`](https://nodejs.org/api/async_hooks.html): what each
//! asynchronous resource is, as Node tracks it, and a store each one it
//! makes carries on, an [`AsyncLocalStorage`]: React's servers keep the
//! request rendering in one, and Flight's debug build tracks each promise
//! awaited by a hook.
//!
//! ```rust,ignore
//! use node::async_hooks::{self, AsyncLocalStorage, HookCallbacks};
//!
//! let storage: &AsyncLocalStorage<Request> = async_hooks::async_local_storage::new(None);
//! storage.run(request, || render(storage.get_store()));
//! async_hooks::create_hook(&HookCallbacks { destroy: Some(&forget), ..Default::default() }).enable();
//! ```
//!
//! ```js
//! import { AsyncLocalStorage, createHook } from "async_hooks";
//!
//! const storage = new AsyncLocalStorage();
//! storage.run(request, () => render(storage.getStore()));
//! createHook({ destroy: forget }).enable();
//! ```

use core::marker::PhantomData;

use js::{JsError, JsObject, Unknown};

/// [`executionAsyncId()`](https://nodejs.org/api/async_hooks.html#async_hooksexecutionasyncid):
/// the id of the resource whose code runs now.
#[cfg_attr(rust_js, rust_js::link_name = "async_hooks#executionAsyncId")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
pub fn execution_async_id() -> f64 {
    unreachable!()
}

/// [`executionAsyncResource()`](https://nodejs.org/api/async_hooks.html#async_hooksexecutionasyncresource):
/// the resource whose code runs now, an object.
#[cfg_attr(rust_js, rust_js::link_name = "async_hooks#executionAsyncResource")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
pub fn execution_async_resource() -> &'static JsObject {
    unreachable!()
}

/// [`triggerAsyncId()`](https://nodejs.org/api/async_hooks.html#async_hookstriggerasyncid):
/// the id of the resource that made the one whose code runs now.
#[cfg_attr(rust_js, rust_js::link_name = "async_hooks#triggerAsyncId")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
pub fn trigger_async_id() -> f64 {
    unreachable!()
}

/// What [`create_hook`] is given, @types/node's `HookCallbacks`: each a
/// function Node calls of each resource as it's made, before and after its
/// code runs, as a promise resolves and as it's gone, by its id, and
/// whether promises are tracked; `None` left out. Node keeps each for as
/// long as the hook is.
#[derive(Clone, Copy, Default)]
pub struct HookCallbacks {
    /// Of a resource made: its id, its type, `"PROMISE"` say, the id of
    /// the one whose code made it, and the resource itself.
    pub init: Option<&'static dyn Fn(f64, &str, f64, &'static JsObject)>,
    pub before: Option<&'static dyn Fn(f64)>,
    pub after: Option<&'static dyn Fn(f64)>,
    pub promise_resolve: Option<&'static dyn Fn(f64)>,
    pub destroy: Option<&'static dyn Fn(f64)>,
    pub track_promises: Option<bool>,
}

/// [`AsyncHook`](https://nodejs.org/api/async_hooks.html#class-asynchook):
/// [`create_hook`]'s, which calls its callbacks while it's enabled.
#[cfg_attr(rust_js, rust_js::types = "async_hooks#AsyncHook")]
pub struct AsyncHook(PhantomData<JsObject>);

impl AsyncHook {
    /// [`hook.enable()`](https://nodejs.org/api/async_hooks.html#asynchookenable):
    /// its callbacks called from now on.
    #[cfg_attr(rust_js, rust_js::link_name = "enable")]
    pub fn enable(&self) -> &Self {
        unreachable!()
    }

    /// [`hook.disable()`](https://nodejs.org/api/async_hooks.html#asynchookdisable):
    /// its callbacks called no more.
    #[cfg_attr(rust_js, rust_js::link_name = "disable")]
    pub fn disable(&self) -> &Self {
        unreachable!()
    }
}

/// [`createHook(callbacks)`](https://nodejs.org/api/async_hooks.html#async_hookscreatehookcallbacks):
/// a hook of `callbacks`, enabled by its [`enable`](AsyncHook::enable).
#[cfg_attr(rust_js, rust_js::link_name = "async_hooks#createHook")]
pub fn create_hook(callbacks: &HookCallbacks) -> &'static AsyncHook {
    unreachable!()
}

/// What [`async_resource::new`] is given after its type, @types/node's
/// `number | AsyncResourceOptions`: the id of the resource that made it,
/// or [`AsyncResourceOptions`].
#[diagnostic::on_unimplemented(message = "`{Self}` is not an async id nor `AsyncResourceOptions`")]
pub trait IntoAsyncResourceTrigger: sealed::Sealed {}
impl IntoAsyncResourceTrigger for f64 {}
impl IntoAsyncResourceTrigger for &AsyncResourceOptions {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for f64 {}
    impl Sealed for &super::AsyncResourceOptions {}
}

/// @types/node's `AsyncResourceOptions`: the id of the resource that made
/// it, and whether its `destroy` hook waits for [`AsyncResource::emit_destroy`].
#[derive(Clone, Copy, Default)]
pub struct AsyncResourceOptions {
    pub trigger_async_id: Option<f64>,
    pub require_manual_destroy: Option<bool>,
}

/// [`AsyncResource`](https://nodejs.org/api/async_hooks.html#class-asyncresource):
/// a resource of a program's own, which hooks see as Node's.
#[cfg_attr(rust_js, rust_js::types = "async_hooks#AsyncResource")]
pub struct AsyncResource(PhantomData<JsObject>);

pub mod async_resource {
    use super::{AsyncResource, IntoAsyncResourceTrigger, JsError};

    /// [`new AsyncResource(type, triggerAsyncId)`](https://nodejs.org/api/async_hooks.html#new-asyncresourcetype-options):
    /// a resource of `type`, made by the one whose code runs now, or by
    /// `trigger`'s.
    #[cfg_attr(rust_js, rust_js::link_name = "new async_hooks#AsyncResource")]
    pub fn new(r#type: &str, trigger: Option<&dyn IntoAsyncResourceTrigger>) -> &'static AsyncResource {
        unreachable!()
    }

    /// `AsyncResource.prototype.asyncId.call(value)`: the id Node gave
    /// `value`, a resource of any kind, a promise too, which has no
    /// `asyncId` of its own, as React's Flight reads a promise's; an `Err`
    /// where `value` is none Node knows, a proxy that throws say.
    #[cfg_attr(rust_js, rust_js::link_name = "async_hooks#AsyncResource.prototype.asyncId.call")]
    pub fn async_id_of(value: &js::Unknown) -> Result<Option<f64>, &'static JsError> {
        unreachable!()
    }
}

impl AsyncResource {
    /// [`resource.asyncId()`](https://nodejs.org/api/async_hooks.html#asyncresourceasyncid):
    /// its id.
    #[cfg_attr(rust_js, rust_js::link_name = "asyncId")]
    pub fn async_id(&self) -> f64 {
        unreachable!()
    }

    /// [`resource.triggerAsyncId()`](https://nodejs.org/api/async_hooks.html#asyncresourcetriggerasyncid):
    /// the id of the resource that made it.
    #[cfg_attr(rust_js, rust_js::link_name = "triggerAsyncId")]
    pub fn trigger_async_id(&self) -> f64 {
        unreachable!()
    }

    /// [`resource.emitDestroy()`](https://nodejs.org/api/async_hooks.html#asyncresourceemitdestroy):
    /// its `destroy` hooks called.
    #[cfg_attr(rust_js, rust_js::link_name = "emitDestroy")]
    pub fn emit_destroy(&self) -> &Self {
        unreachable!()
    }

    /// [`resource.runInAsyncScope(fn)`](https://nodejs.org/api/async_hooks.html#asyncresourceruninasyncscopefn-thisarg-args):
    /// what `f` gives, run as the resource's code.
    #[cfg_attr(rust_js, rust_js::link_name = "runInAsyncScope")]
    pub fn run_in_async_scope<R>(&self, f: impl FnOnce() -> R) -> R {
        unreachable!()
    }
}

/// @types/node's `AsyncLocalStorageOptions`: what a store is where none
/// was given, and its name.
#[derive(Clone, Copy, Default)]
pub struct AsyncLocalStorageOptions<'a> {
    pub default_value: Option<&'a Unknown>,
    pub name: Option<&'a str>,
}

/// [`AsyncLocalStorage<T>`](https://nodejs.org/api/async_context.html#class-asynclocalstorage):
/// a store of a `T`, which the code [`run`](AsyncLocalStorage::run) of it
/// runs, and each resource that code makes, reads.
#[cfg_attr(rust_js, rust_js::types = "async_hooks#AsyncLocalStorage")]
pub struct AsyncLocalStorage<T: ?Sized>(PhantomData<(JsObject, *const T)>);

pub mod async_local_storage {
    use super::{AsyncLocalStorage, AsyncLocalStorageOptions};

    /// [`new AsyncLocalStorage(options)`](https://nodejs.org/api/async_context.html#new-asynclocalstorage):
    /// a storage of no store.
    #[cfg_attr(rust_js, rust_js::link_name = "new async_hooks#AsyncLocalStorage")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn new<T: ?Sized>(options: Option<&AsyncLocalStorageOptions<'_>>) -> &'static AsyncLocalStorage<T> {
        unreachable!()
    }
}

impl<T: ?Sized> AsyncLocalStorage<T> {
    /// [`storage.getStore()`](https://nodejs.org/api/async_context.html#asynclocalstoragegetstore):
    /// the store of the code that runs now, `None` outside a
    /// [`run`](Self::run).
    #[cfg_attr(rust_js, rust_js::link_name = "getStore")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn get_store(&self) -> Option<&'static T> {
        unreachable!()
    }

    /// [`storage.run(store, callback)`](https://nodejs.org/api/async_context.html#asynclocalstoragerunstore-callback-args):
    /// what `callback` gives, run with `store` the store, as is each
    /// resource it makes.
    #[cfg_attr(rust_js, rust_js::link_name = "run")]
    pub fn run<R>(&self, store: &'static T, callback: impl FnOnce() -> R) -> R {
        unreachable!()
    }

    /// [`storage.exit(callback)`](https://nodejs.org/api/async_context.html#asynclocalstorageexitcallback-args):
    /// what `callback` gives, run with no store.
    #[cfg_attr(rust_js, rust_js::link_name = "exit")]
    pub fn exit<R>(&self, callback: impl FnOnce() -> R) -> R {
        unreachable!()
    }

    /// [`storage.enterWith(store)`](https://nodejs.org/api/async_context.html#asynclocalstorageenterwithstore):
    /// `store` the store of the code that runs from now on.
    #[cfg_attr(rust_js, rust_js::link_name = "enterWith")]
    pub fn enter_with(&self, store: &'static T) {
        unreachable!()
    }

    /// [`storage.disable()`](https://nodejs.org/api/async_context.html#asynclocalstoragedisable):
    /// no store from now on, until a [`run`](Self::run).
    #[cfg_attr(rust_js, rust_js::link_name = "disable")]
    pub fn disable(&self) {
        unreachable!()
    }

    /// [`storage.name`](https://nodejs.org/api/async_context.html#asynclocalstoragename):
    /// its name, its options' or `""`.
    #[cfg_attr(rust_js, rust_js::link_name = "get name")]
    pub fn name(&self) -> String {
        unreachable!()
    }
}
