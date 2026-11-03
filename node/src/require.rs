//! [`require`](https://nodejs.org/api/modules.html#requireid): a CommonJS
//! module's own `require`, @types/node's `NodeJS.Require`, of which
//! [`resolve`] finds a module's file without loading it. An ES module has
//! none, so a call of it there throws a `ReferenceError`, a panic, as a
//! read of [`crate::module`] does (ADR 0305, amended).

/// [`require.resolve(request, options)`](https://nodejs.org/api/modules.html#requireresolverequest-options):
/// the full path of the file `request` names, as `require(request)` would
/// find it from the module the code is in, or from `options`' `paths`
/// where it gives them; `None` is left out. Of a module it can't find, its
/// `MODULE_NOT_FOUND` error, a panic.
#[cfg_attr(rust_js, rust_js::link_name = "require.resolve")]
pub fn resolve(request: &str, options: Option<&RequireResolveOptions<'_>>) -> String {
    unreachable!()
}

/// What [`resolve`] is given, @types/node's `RequireResolveOptions`, a
/// `None` left out.
#[derive(Clone, Copy, Default)]
pub struct RequireResolveOptions<'a> {
    /// The directories to look from, in place of the module's own.
    pub paths: Option<&'a [String]>,
}
