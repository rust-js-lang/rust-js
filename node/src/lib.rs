//! [Node.js](https://nodejs.org) for rust-js (ADR 0272): its modules, as
//! `@types/node` types them, for a program's code that runs in Node.
//!
//! ```rust,ignore
//! use node::{BufferEncoding, fs, process};
//!
//! let root = process::cwd() + "/src/content";
//! let text = fs::read_file_sync(&(root + "/index.md"), BufferEncoding::Utf8);
//! ```
//!
//! ```js
//! import { readFileSync } from "fs";
//!
//! const root = process.cwd() + "/src/content";
//! const text = readFileSync(root + "/index.md", "utf8");
//! ```

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

pub mod fs;
pub mod http;
pub mod path;
pub mod process;
pub mod url;

pub use webapi;

/// [How text is bytes](https://nodejs.org/api/buffer.html#buffers-and-character-encodings),
/// `@types/node`'s `BufferEncoding`: what a file's text is read as.
pub enum BufferEncoding {
    #[cfg_attr(rust_js, rust_js::name = "ascii")]
    Ascii,
    #[cfg_attr(rust_js, rust_js::name = "utf8")]
    Utf8,
    #[cfg_attr(rust_js, rust_js::name = "utf-8")]
    Utf8Dash,
    #[cfg_attr(rust_js, rust_js::name = "utf16le")]
    Utf16le,
    #[cfg_attr(rust_js, rust_js::name = "ucs2")]
    Ucs2,
    #[cfg_attr(rust_js, rust_js::name = "ucs-2")]
    Ucs2Dash,
    #[cfg_attr(rust_js, rust_js::name = "base64")]
    Base64,
    #[cfg_attr(rust_js, rust_js::name = "base64url")]
    Base64url,
    #[cfg_attr(rust_js, rust_js::name = "latin1")]
    Latin1,
    #[cfg_attr(rust_js, rust_js::name = "binary")]
    Binary,
    #[cfg_attr(rust_js, rust_js::name = "hex")]
    Hex,
}
