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
// Many Rust functions call the same JS name: a form per optional argument
// (`basename`, `basename_with_suffix`). rustc warns because in native code
// they would be one symbol.
#![allow(clashing_extern_declarations)]
// And some are named as libc's functions are, `abort`, `exit` and `kill`:
// JS functions, which in native code rustc would take for those.
#![allow(invalid_runtime_symbol_definitions)]

pub mod buffer;
pub mod event;
pub mod events;
pub mod fs;
pub mod http;
pub mod js_error;
pub mod path;
pub mod process;
pub mod querystring;
pub mod url;
pub mod util;

pub use js;
pub use webapi;

use core::marker::PhantomData;

use js::{JsObject, Unknown};

/// [A frame of a stack](https://v8.dev/docs/stack-trace-api#customizing-stack-traces),
/// `NodeJS.CallSite`: what [`js_error::prepare_stack_trace`]'s hook is
/// given, one for each call the error was made in (ADR 0648).
pub struct CallSite(PhantomData<JsObject>);

impl CallSite {
    /// `callSite.getColumnNumber()`: its column, from 1, `None`, `null`, where it has none.
    #[cfg_attr(rust_js, rust_js::link_name = "getColumnNumber")]
    pub fn get_column_number(&self) -> Option<u32> {
        unreachable!()
    }

    /// `callSite.getEnclosingColumnNumber()`: the column its function starts at, from 1.
    #[cfg_attr(rust_js, rust_js::link_name = "getEnclosingColumnNumber")]
    pub fn get_enclosing_column_number(&self) -> Option<u32> {
        unreachable!()
    }

    /// `callSite.getEnclosingLineNumber()`: the line its function starts on, from 1.
    #[cfg_attr(rust_js, rust_js::link_name = "getEnclosingLineNumber")]
    pub fn get_enclosing_line_number(&self) -> Option<u32> {
        unreachable!()
    }

    /// `callSite.getEvalOrigin()`: where the `eval` it ran in was called, `None` outside one.
    #[cfg_attr(rust_js, rust_js::link_name = "getEvalOrigin")]
    pub fn get_eval_origin(&self) -> Option<String> {
        unreachable!()
    }

    /// `callSite.getFileName()`: its script's name or URL.
    #[cfg_attr(rust_js, rust_js::link_name = "getFileName")]
    pub fn get_file_name(&self) -> Option<String> {
        unreachable!()
    }

    /// `callSite.getFunction()`: its function, `None` in strict mode, of
    /// whatever it takes, TypeScript's `Function` (ADR 0653).
    #[cfg_attr(rust_js, rust_js::link_name = "getFunction")]
    pub fn get_function(&self) -> Option<&'static dyn Fn()> {
        unreachable!()
    }

    /// `callSite.getFunctionName()`: its function's name, `None` of one without.
    #[cfg_attr(rust_js, rust_js::link_name = "getFunctionName")]
    pub fn get_function_name(&self) -> Option<String> {
        unreachable!()
    }

    /// `callSite.getLineNumber()`: its line, from 1.
    #[cfg_attr(rust_js, rust_js::link_name = "getLineNumber")]
    pub fn get_line_number(&self) -> Option<u32> {
        unreachable!()
    }

    /// `callSite.getMethodName()`: the name of the property of `this` its function is.
    #[cfg_attr(rust_js, rust_js::link_name = "getMethodName")]
    pub fn get_method_name(&self) -> Option<String> {
        unreachable!()
    }

    /// `callSite.getPosition()`: its offset in its script's source.
    #[cfg_attr(rust_js, rust_js::link_name = "getPosition")]
    pub fn get_position(&self) -> u32 {
        unreachable!()
    }

    /// `callSite.getPromiseIndex()`: its promise's index in `Promise.all`'s, of one of its frames.
    #[cfg_attr(rust_js, rust_js::link_name = "getPromiseIndex")]
    pub fn get_promise_index(&self) -> Option<u32> {
        unreachable!()
    }

    /// `callSite.getScriptHash()`: its script's hash.
    #[cfg_attr(rust_js, rust_js::link_name = "getScriptHash")]
    pub fn get_script_hash(&self) -> String {
        unreachable!()
    }

    /// `callSite.getScriptNameOrSourceURL()`: its script's name, or its `//# sourceURL`.
    #[cfg_attr(rust_js, rust_js::link_name = "getScriptNameOrSourceURL")]
    pub fn get_script_name_or_source_url(&self) -> Option<String> {
        unreachable!()
    }

    /// `callSite.getThis()`: its `this`, of any shape, `None` in strict mode.
    #[cfg_attr(rust_js, rust_js::link_name = "getThis")]
    pub fn get_this(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    /// `callSite.getTypeName()`: the type of its `this`, its constructor's name.
    #[cfg_attr(rust_js, rust_js::link_name = "getTypeName")]
    pub fn get_type_name(&self) -> Option<String> {
        unreachable!()
    }

    /// `callSite.isAsync()`: whether it's an `await`'s.
    #[cfg_attr(rust_js, rust_js::link_name = "isAsync")]
    pub fn is_async(&self) -> bool {
        unreachable!()
    }

    /// `callSite.isConstructor()`: whether it's a `new`'s.
    #[cfg_attr(rust_js, rust_js::link_name = "isConstructor")]
    pub fn is_constructor(&self) -> bool {
        unreachable!()
    }

    /// `callSite.isEval()`: whether it's in an `eval`'s code.
    #[cfg_attr(rust_js, rust_js::link_name = "isEval")]
    pub fn is_eval(&self) -> bool {
        unreachable!()
    }

    /// `callSite.isNative()`: whether it's in V8's own code.
    #[cfg_attr(rust_js, rust_js::link_name = "isNative")]
    pub fn is_native(&self) -> bool {
        unreachable!()
    }

    /// `callSite.isPromiseAll()`: whether it's `Promise.all`'s.
    #[cfg_attr(rust_js, rust_js::link_name = "isPromiseAll")]
    pub fn is_promise_all(&self) -> bool {
        unreachable!()
    }

    /// `callSite.isToplevel()`: whether its `this` is the global object, a script's or a module's own code.
    #[cfg_attr(rust_js, rust_js::link_name = "isToplevel")]
    pub fn is_toplevel(&self) -> bool {
        unreachable!()
    }

    /// `callSite.toString()`: the frame as V8 writes it in a stack,
    /// `render (App.js:3:11)`, `Object`'s method to TypeScript.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }
}

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
    #[cfg_attr(rust_js, rust_js::name = "utf-16le")]
    Utf16leDash,
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
