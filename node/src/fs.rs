//! [`fs`](https://nodejs.org/api/fs.html): files.

use js::JsError;

use crate::BufferEncoding;

unsafe extern "Rust" {
    /// [`readFileSync(path, encoding)`](https://nodejs.org/api/fs.html#fsreadfilesyncpath-options):
    /// the file's text, read as `encoding`; what it throws, the file missing
    /// say, an `Err`.
    #[link_name = "fs#readFileSync"]
    pub safe fn read_file_sync(path: &str, options: BufferEncoding) -> Result<String, &'static JsError>;
}

/// What a `PathLike` parameter, `string | Buffer | URL`, takes: each as it
/// is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a path, a `&str` or a `&URL`")]
#[cfg_attr(rust_js, rust_js::types = "PathLike")]
pub trait IntoPathLike: sealed::Sealed {}
impl IntoPathLike for &str {}
impl IntoPathLike for &String {}
impl IntoPathLike for &webapi::URL {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &String {}
    impl Sealed for &webapi::URL {}
}
