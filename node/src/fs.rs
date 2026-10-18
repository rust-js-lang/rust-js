//! [`fs`](https://nodejs.org/api/fs.html): files.

use js::JsError;

use crate::BufferEncoding;

unsafe extern "Rust" {

    /// [`readdir(path, callback)`](https://nodejs.org/api/fs.html#fsreaddirpath-options-callback):
    /// `callback` given the names in the directory `path`, or what failed.
    #[link_name = "fs#readdir"]
    pub safe static readdir: for<'a> fn(&'a str, Box<dyn FnOnce(Option<&'static JsError>, Vec<String>)>);

    /// [`stat(path, callback)`](https://nodejs.org/api/fs.html#fsstatpath-options-callback):
    /// `callback` given what `path` is, a file or a directory, or what failed.
    #[link_name = "fs#stat"]
    pub safe static stat: for<'a> fn(&'a str, Box<dyn FnOnce(Option<&'static JsError>, &'static Stats)>);
}

/// [`readFileSync(path, encoding)`](https://nodejs.org/api/fs.html#fsreadfilesyncpath-options):
/// the file's text, read as `encoding`; what it throws, the file missing
/// say, an `Err`.
#[cfg_attr(rust_js, rust_js::link_name = "fs#readFileSync")]
pub fn read_file_sync(path: &str, options: BufferEncoding) -> Result<String, &'static JsError> {
    unreachable!()
}

/// [`fs.Stats`](https://nodejs.org/api/fs.html#class-fsstats): what
/// [`stat`] says of a path.
#[cfg_attr(rust_js, rust_js::types = "fs#Stats")]
pub struct Stats(core::marker::PhantomData<js::JsObject>);

impl Stats {
    /// [`stats.isDirectory()`](https://nodejs.org/api/fs.html#statsisdirectory):
    /// whether it's a directory.
    #[cfg_attr(rust_js, rust_js::link_name = "isDirectory")]
    pub fn is_directory(&self) -> bool {
        unreachable!()
    }

    /// [`stats.isFile()`](https://nodejs.org/api/fs.html#statsisfile):
    /// whether it's a file.
    #[cfg_attr(rust_js, rust_js::link_name = "isFile")]
    pub fn is_file(&self) -> bool {
        unreachable!()
    }
}

/// What a `PathLike` parameter, `string | Buffer | URL`, takes: each as it
/// is (ADR 0229); an optional one, `Option<&dyn IntoPathLike>`, borrowed,
/// `Some(&value)` (ADR 0443).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a path, a `&str`, a `&Buffer` or a `&URL`",
    note = "an optional one, `Option<&dyn IntoPathLike>`, takes `Some(&value)`"
)]
#[cfg_attr(rust_js, rust_js::types = "PathLike")]
pub trait IntoPathLike: sealed::Sealed {}
impl IntoPathLike for &str {}
impl IntoPathLike for &String {}
impl IntoPathLike for &webapi::URL {}
impl IntoPathLike for &crate::buffer::Buffer {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &String {}
    impl Sealed for &webapi::URL {}
    impl Sealed for &crate::buffer::Buffer {}
}
