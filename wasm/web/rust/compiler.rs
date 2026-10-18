// What the page downloads, and running rust-js.wasm on it.
//
// A Rust crate in (a few files), one JS file per module out (ADR 0019).
// rust-js.wasm runs on an in-memory WASI filesystem:
//
//   /in/lib.rs, /in/stats.rs, ...   the crate, from the Rust editor
//   /out/lib.js, /out/stats.js, ... what rust-js writes (plus .js.map files)
//   /sysroot/...                    the std metadata rustc type-checks against
//   /crates/libreact.rmeta, ...     each crate a program may use, its
//                                   metadata: react's, and those it uses,
//                                   webapi's and js's (ADRs 0024, 0102)
//
// Each compile gets a fresh instance of the (compiled once) module: rustc
// keeps global state, and a failed compile ends in a trap.

use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use js::{JsError, JsObject, Promise, Uint8Array, js_error, number, uint8_array};
use webapi::{
    Response, WebAssemblyInstance, WebAssemblyMemory, WebAssemblyModule, performance, text_decoder, text_encoder,
    web_assembly, window,
};

// The crates a program may use, each once, as the site serves them and its
// build compiles against (ADR 0102).
unsafe extern "Rust" {
    #[link_name = "../../../tooling/crates.js#reactLibs"]
    safe static react_libs: &'static [&'static str];
}

/// A file in the WASI shim's in-memory filesystem.
pub struct WasiFile(PhantomData<JsObject>);
/// A file or a directory: what a directory's `Map` holds.
pub struct Inode(PhantomData<JsObject>);
pub struct WasiDirectory(PhantomData<JsObject>);
pub struct PreopenDirectory(PhantomData<JsObject>);
/// One of WASI's file descriptors: stdin, stdout, a preopened directory.
pub struct Fd(PhantomData<JsObject>);
pub struct Wasi(PhantomData<JsObject>);

// Some JS functions are declared more than once, typed for each use (`json`
// for each file it reads, `new Directory` for what it holds): rustc warns
// that native code would see one symbol. A directory's contents are a JS
// `Map`, which a `HashMap` is.
#[cfg_attr(rust_js, rust_js::link_name = "new @bjorn3/browser_wasi_shim#File")]
fn new_file(data: &Uint8Array, options: &dyn std::any::Any) -> &'static WasiFile {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get data")]
fn file_data(this: &WasiFile) -> &'static Uint8Array {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "json")]
fn names_json(this: &Response) -> Promise<Vec<String>> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "json")]
fn examples_json(this: &Response) -> Promise<Vec<Example>> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "json")]
fn packages_json(this: &Response) -> Promise<Packages> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
fn manifest_of(text: &str) -> Manifest {
    unreachable!()
}

/// An object's fields, as `(name, text)`: `packages.json`'s modules.
#[cfg_attr(rust_js, rust_js::link_name = "Object.entries")]
fn text_fields(object: &JsObject) -> Vec<(String, String)> {
    unreachable!()
}

// The rest of the WASI shim (@bjorn3/browser_wasi_shim), for `compile`.
#[cfg_attr(rust_js, rust_js::link_name = "new @bjorn3/browser_wasi_shim#File")]
fn new_empty_file(data: Vec<u8>) -> &'static WasiFile {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new @bjorn3/browser_wasi_shim#File")]
fn new_plain_file(data: &Uint8Array) -> &'static WasiFile {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new @bjorn3/browser_wasi_shim#Directory")]
fn new_directory(contents: &HashMap<String, &'static Inode>) -> &'static WasiDirectory {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new @bjorn3/browser_wasi_shim#Directory")]
fn new_file_directory(contents: &HashMap<String, &'static WasiFile>) -> &'static WasiDirectory {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get contents")]
fn contents(this: &WasiDirectory) -> &'static mut HashMap<String, &'static Inode> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "this")]
fn file_inode(this: &WasiFile) -> &'static Inode {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "this")]
fn directory_inode(this: &WasiDirectory) -> &'static Inode {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "instanceof @bjorn3/browser_wasi_shim#Directory")]
fn is_directory(this: &Inode) -> bool {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "instanceof @bjorn3/browser_wasi_shim#File")]
fn is_file(this: &Inode) -> bool {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "this")]
fn as_directory(this: &Inode) -> &'static WasiDirectory {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "this")]
fn as_file(this: &Inode) -> &'static WasiFile {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new @bjorn3/browser_wasi_shim#OpenFile")]
fn new_open_file(file: &WasiFile) -> &'static Fd {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "@bjorn3/browser_wasi_shim#ConsoleStdout.lineBuffered")]
fn line_buffered(write: Box<dyn FnMut(String)>) -> &'static Fd {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new @bjorn3/browser_wasi_shim#PreopenDirectory")]
fn new_preopen(name: &str, contents: &HashMap<String, &'static Inode>) -> &'static PreopenDirectory {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "this")]
fn preopen_fd(this: &PreopenDirectory) -> &'static Fd {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get dir")]
fn preopen_dir(this: &PreopenDirectory) -> &'static WasiDirectory {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new @bjorn3/browser_wasi_shim#WASI")]
fn new_wasi(args: Vec<String>, env: Vec<String>, fds: Vec<&'static Fd>, options: &dyn std::any::Any) -> &'static Wasi {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get wasiImport")]
fn wasi_import(this: &Wasi) -> &'static JsObject {
    unreachable!()
}

/// Runs the program. A failed compile ends in a trap: panics can't unwind
/// on wasm32-wasip1.
#[cfg_attr(rust_js, rust_js::link_name = "start")]
fn run_wasi(this: &Wasi, instance: &WebAssemblyInstance) -> Result<i32, &'static JsError> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get memory")]
fn exported_memory(this: &JsObject) -> &'static WebAssemblyMemory {
    unreachable!()
}

/// The last compile, for automated checks.
#[cfg_attr(rust_js, rust_js::link_name = "set lastResult")]
pub fn set_last_result(this: &webapi::Window, result: &Compiled) {
    unreachable!()
}

/// An example program: its files, under `examples/<name>/`.
pub struct Example {
    pub name: String,
    pub title: String,
    pub root: String,
    pub files: Vec<String>,
}

/// `packages.json`: what a program may import of the page's, by specifier.
struct Packages {
    modules: &'static JsObject,
    styles: &'static JsObject,
}

/// What rust-js says it wrote (ADR 0091): each module's JS, what each
/// exports, and its tests' runner. The playground reads it, not the JS.
struct Manifest {
    modules: Vec<ManifestModule>,
    tests: Option<String>,
}

struct ManifestModule {
    module: Vec<String>,
    file: String,
    exports: Vec<String>,
}

/// What the page needs before it can compile anything.
pub struct Loaded {
    pub module: &'static WebAssemblyModule,
    pub sysroot: HashMap<String, &'static WasiFile>,
    /// Each crate a program may use, by its library's name, its metadata.
    pub crates: Vec<(String, &'static WasiFile)>,
    /// What the programs it compiles import of the page's, by specifier:
    /// `@rust-js/runtime` (ADR 0103), and React's modules.
    pub modules: Vec<(String, String)>,
    /// The stylesheets a program may import, TodoMVC's, by specifier.
    pub styles: Vec<(String, String)>,
    pub examples: Vec<Example>,
}

/// A row of the stats table: what was measured, and how long it took.
pub type Stat = Rc<dyn Fn(String, String)>;

/// The WASI shim's file options. Only the shim reads them.
struct FileOptions {
    readonly: bool,
}

pub fn ms(t: f64) -> String {
    format!("{} ms", number::to_fixed(t, 0))
}

pub fn mb(n: f64) -> String {
    format!("{} MB", number::to_fixed(n / 1048576.0, 1))
}

/// Download the compiler, the sysroot, the webapi and js crates and the examples,
/// giving `stat` each one's time as it arrives.
pub async fn load(stat: Stat) -> Loaded {
    let start = performance.now();
    // All downloads start here, together: a JS promise runs as soon as it's made
    // (ADR 0029). Awaiting them one by one below only collects the results.
    let module = load_compiler(start, stat.clone());
    let sysroot = load_sysroot(start, stat.clone());
    let crates: Vec<_> = (react_libs.iter())
        .map(|name| (name.to_string(), load_binding_crate(name, start, stat.clone())))
        .collect();
    let packages = load_packages(start, stat.clone());
    let examples = load_examples();
    let (modules, styles) = packages.await;
    let mut crate_files = Vec::new();
    for (name, file) in crates {
        crate_files.push((name, file.await));
    }
    let loaded = Loaded {
        module: module.await,
        sysroot: sysroot.await,
        crates: crate_files,
        modules,
        styles,
        examples: examples.await,
    };
    stat("ready after".to_string(), ms(performance.now() - start));
    loaded
}

async fn load_compiler(start: f64, stat: Stat) -> &'static WebAssemblyModule {
    let module = web_assembly::compile_streaming(window.fetch("./rust-js.wasm", None), None).await;
    stat(
        "download + compile rust-js.wasm".to_string(),
        ms(performance.now() - start),
    );
    module
}

async fn load_sysroot(start: f64, stat: Stat) -> HashMap<String, &'static WasiFile> {
    let names = names_json(window.fetch("./sysroot.json", None).await).await;
    // Every file's download starts before the first is awaited.
    let mut downloads = Vec::new();
    for name in names {
        downloads.push(load_sysroot_file(name));
    }
    let mut entries = Vec::new();
    let mut size = 0;
    for download in downloads {
        let entry = download.await;
        size += file_data(entry.1).length();
        entries.push(entry);
    }
    stat(
        "download sysroot".to_string(),
        format!(
            "{} ({} files, {})",
            ms(performance.now() - start),
            entries.len(),
            mb(size as f64)
        ),
    );
    entries.into_iter().collect()
}

async fn load_sysroot_file(name: String) -> (String, &'static WasiFile) {
    let response = window.fetch(format!("./sysroot/{name}").as_str(), None).await;
    let bytes = uint8_array::new_with_buffer(response.array_buffer().await, None, None);
    (name, new_file(bytes, &FileOptions { readonly: true }))
}

/// `@rust-js/runtime` and React's modules, and the stylesheets, for a
/// program to import (ADR 0044).
async fn load_packages(start: f64, stat: Stat) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let runtime = window.fetch("./runtime.js", None).await.text();
    let packages = packages_json(window.fetch("./packages.json", None).await);
    let mut modules = vec![("@rust-js/runtime".to_string(), runtime.await)];
    let packages = packages.await;
    modules.extend(text_fields(packages.modules));
    stat(
        "download runtime and packages".to_string(),
        ms(performance.now() - start),
    );
    (modules, text_fields(packages.styles))
}

async fn load_binding_crate(name: &str, start: f64, stat: Stat) -> &'static WasiFile {
    let bytes = (window.fetch(format!("./crates/lib{name}.rmeta").as_str(), None).await)
        .array_buffer()
        .await;
    stat(
        format!("download {name} crate"),
        format!("{} ({})", ms(performance.now() - start), mb(bytes.byte_length() as f64)),
    );
    new_file(
        uint8_array::new_with_buffer(bytes, None, None),
        &FileOptions { readonly: true },
    )
}

async fn load_examples() -> Vec<Example> {
    examples_json(window.fetch("./examples.json", None).await).await
}

async fn fetch_example_file(name: String, path: String) -> (String, String) {
    let response = window.fetch(format!("./examples/{name}/{path}").as_str(), None).await;
    (path, response.text().await)
}

/// An example's files, `path → text`, in the order it lists them.
pub async fn load_example(name: String, paths: Vec<String>) -> Vec<(String, String)> {
    // All at once: each download starts as it's made (ADR 0029).
    let mut downloads = Vec::new();
    for path in paths {
        downloads.push(fetch_example_file(name.clone(), path));
    }
    let mut texts = Vec::new();
    for download in downloads {
        texts.push(download.await);
    }
    texts
}

// ── Running rust-js ─────────────────────────────────────────────────────

/// What a compile gives back. `exit` is its code, or how it trapped.
pub struct Compiled {
    pub exit: String,
    pub ok: bool,
    /// The JS files, `path → text`.
    pub files: HashMap<String, String>,
    /// The root module's, `lib.js` or `lib.jsx`.
    pub root: String,
    /// Whether it exports `main`, what running the program calls.
    pub main: bool,
    /// The `#[test]` functions' runner, of a compile with `test`.
    pub tests: Option<String>,
    pub stderr: String,
    pub instantiate: f64,
    pub run: f64,
    pub memory: u32,
}

/// The WASI shim's options, and the module's imports. Only JS reads them.
struct WasiOptions {
    debug: bool,
}

struct Imports {
    /// WASI's own name for its imports.
    #[cfg_attr(rust_js, rust_js::name = "wasi_snapshot_preview1")]
    wasi_snapshot_preview1: &'static JsObject,
}

/// A directory holding one entry.
fn dir(name: &str, entry: &'static Inode) -> &'static Inode {
    directory_inode(new_directory(&HashMap::from([(name.to_string(), entry)])))
}

/// A WASI directory tree from `path → text`, e.g. `geometry/area.rs`.
fn directory_of(sources: &HashMap<String, String>) -> HashMap<String, &'static Inode> {
    let mut top = HashMap::new();
    for (path, text) in sources {
        let mut folder = &mut top;
        let name = match path.rsplit_once('/') {
            Some((folders, name)) => {
                for part in folders.split('/') {
                    if !folder.contains_key(part) {
                        folder.insert(part.to_string(), directory_inode(new_directory(&HashMap::new())));
                    }
                    folder = contents(as_directory(folder[part]));
                }
                name
            }
            None => path.as_str(),
        };
        let bytes = text_encoder::new().encode(Some(text));
        folder.insert(name.to_string(), file_inode(new_plain_file(bytes)));
    }
    top
}

/// Every file under a WASI directory, as `path → text`.
fn texts_in(folder: &WasiDirectory, prefix: &str, found: &mut HashMap<String, String>) {
    for (name, &entry) in contents(folder).iter() {
        if is_directory(entry) {
            texts_in(as_directory(entry), &format!("{prefix}{name}/"), found);
        } else if is_file(entry) {
            let text = text_decoder::new(None, None).decode(Some(&file_data(as_file(entry))), None);
            found.insert(format!("{prefix}{name}"), text);
        }
    }
}

/// Run rust-js.wasm on the crate in `sources` (`path → text`): a fresh
/// instance each time, since rustc keeps global state, and a failed compile
/// ends in a trap. With `test`, the crate's `#[test]` functions too (ADR 0026).
pub async fn compile(loaded: &Loaded, sources: &HashMap<String, String>, root_file: &str, test: bool) -> Compiled {
    let stderr = Rc::new(RefCell::new(Vec::new()));
    let stdout_lines = stderr.clone();
    let stderr_lines = stderr.clone();
    let out_dir = new_preopen("/out", &HashMap::new());
    let sysroot_dir = directory_inode(new_file_directory(&loaded.sysroot));
    let fds = vec![
        new_open_file(new_empty_file(Vec::new())), // stdin
        line_buffered(Box::new(move |line| stdout_lines.borrow_mut().push(line))), // stdout
        line_buffered(Box::new(move |line| stderr_lines.borrow_mut().push(line))), // stderr
        preopen_fd(new_preopen("/in", &directory_of(sources))),
        preopen_fd(out_dir),
        preopen_fd(new_preopen(
            "/sysroot",
            &HashMap::from([(
                "lib".to_string(),
                dir("rustlib", dir("wasm32-unknown-unknown", dir("lib", sysroot_dir))),
            )]),
        )),
        preopen_fd(new_preopen(
            "/crates",
            &(loaded.crates.iter())
                .map(|(name, file)| (format!("lib{name}.rmeta"), file_inode(file)))
                .collect(),
        )),
    ];
    let out_file = match root_file.strip_suffix(".rs") {
        Some(stem) => format!("/out/{stem}.js"),
        None => format!("/out/{root_file}"),
    };
    // `--test`: the `#[test]` functions too, and `<root>.test.js` to run them
    // (ADR 0026). This is a real browser, so tests marked `#[cfg(browser)]` run
    // too (ADR 0027). Every program may use the webapi and js crates; rustc
    // only reads them if one does.
    let mut args = vec!["rust-js".to_string()];
    if test {
        args.push("--test".to_string());
    }
    for arg in [
        format!("/in/{root_file}"),
        "-o".to_string(),
        out_file,
        "--manifest".to_string(),
        "/out/manifest.json".to_string(),
    ] {
        args.push(arg);
    }
    for arg in ["--", "--target", "wasm32-unknown-unknown", "--sysroot", "/sysroot"] {
        args.push(arg.to_string());
    }
    if test {
        args.push("--cfg=browser".to_string());
    }
    for (name, _) in &loaded.crates {
        args.push("--extern".to_string());
        args.push(format!("{name}=/crates/lib{name}.rmeta"));
    }
    args.push("-L".to_string());
    args.push("/crates".to_string());
    // RUSTC_ICE=0: don't name a crash-report file after the process id (WASI
    // has none). Without options, the shim logs every call it handles.
    let wasi = new_wasi(
        args,
        vec!["RUSTC_ICE=0".to_string()],
        fds,
        &WasiOptions { debug: false },
    );

    let t0 = performance.now();
    let imports = Imports {
        wasi_snapshot_preview1: wasi_import(wasi),
    };
    let instance = web_assembly::instantiate_with_web_assembly_module(loaded.module, Some(&imports)).await;
    let t1 = performance.now();
    let started = run_wasi(wasi, instance);
    let t2 = performance.now();
    let ok = matches!(started, Ok(0));
    let exit = match started {
        Ok(code) => code.to_string(),
        Err(e) => format!(
            "trap ({})",
            if e.is_error() {
                e.message()
            } else {
                js_error::to_string(e)
            }
        ),
    };

    // What it wrote, as its manifest says: each module's JS, and the runner.
    let mut written = HashMap::new();
    if ok {
        texts_in(preopen_dir(out_dir), "", &mut written);
    }
    let path = |file: &str| file.strip_prefix("/out/").unwrap_or(file).to_string();
    let mut files = HashMap::new();
    let (mut root, mut main, mut tests) = (String::new(), false, None);
    if let Some(text) = written.get("manifest.json") {
        let manifest = manifest_of(text);
        for module in &manifest.modules {
            let file = path(&module.file);
            if module.module.is_empty() {
                root = file.clone();
                main = module.exports.iter().any(|name| name == "main");
            }
            if let Some(text) = written.get(&file) {
                files.insert(file, text.clone());
            }
        }
        if let Some(runner) = &manifest.tests {
            let file = path(runner);
            if let Some(text) = written.get(&file) {
                files.insert(file.clone(), text.clone());
            }
            tests = Some(file);
        }
    }
    let memory = exported_memory(instance.exports()).buffer();
    Compiled {
        exit,
        ok,
        files,
        root,
        main,
        tests,
        stderr: stderr.borrow().join("\n"),
        instantiate: t1 - t0,
        run: t2 - t1,
        memory: memory.byte_length(),
    }
}
