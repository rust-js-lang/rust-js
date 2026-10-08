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
