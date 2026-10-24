//! [`module`](https://nodejs.org/api/module.html): Node's modules, of
//! which a CommonJS one's own `module`, [`crate::module`], is a [`Module`].

use core::marker::PhantomData;

use js::{JsObject, Unknown};

/// [A CommonJS module](https://nodejs.org/api/modules.html#the-module-object),
/// `NodeJS.Module`, `module.Module`'s: what [`crate::module`] is, of the
/// module its code is in.
pub struct Module(PhantomData<JsObject>);

// An object, never `undefined`: a value of any shape, whose properties
// `js::get` reads by a name it makes, `module[requireString]`.
unsafe impl js::Defined for Module {}

impl Module {
    /// [`module.id`](https://nodejs.org/api/modules.html#moduleid): its id,
    /// its file's full path, mostly.
    #[cfg_attr(rust_js, rust_js::link_name = "get id")]
    pub fn id(&self) -> String {
        unreachable!()
    }

    /// [`module.filename`](https://nodejs.org/api/modules.html#modulefilename):
    /// its file's full path.
    #[cfg_attr(rust_js, rust_js::link_name = "get filename")]
    pub fn filename(&self) -> String {
        unreachable!()
    }

    /// [`module.loaded`](https://nodejs.org/api/modules.html#moduleloaded):
    /// whether it's done loading.
    #[cfg_attr(rust_js, rust_js::link_name = "get loaded")]
    pub fn loaded(&self) -> bool {
        unreachable!()
    }

    /// [`module.require(id)`](https://nodejs.org/api/modules.html#modulerequireid):
    /// the exports of the module `id` names, as `require(id)` loads them,
    /// any value; what it throws, a module it can't find, a panic.
    #[cfg_attr(rust_js, rust_js::link_name = "require")]
    pub fn require(&self, id: &str) -> Option<&'static Unknown> {
        unreachable!()
    }
}
