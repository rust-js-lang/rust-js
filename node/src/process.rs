//! [`process`](https://nodejs.org/api/process.html): the Node process, a
//! global.

unsafe extern "Rust" {
    /// [`process.cwd()`](https://nodejs.org/api/process.html#processcwd):
    /// the directory the process runs in.
    #[link_name = "process.cwd"]
    pub safe fn cwd() -> String;
}
