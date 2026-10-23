// The node crate: Node.js's modules imported and its globals called,
// its callbacks promisified, its buffers, events, process and URLs as
// Node's own. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildNode, buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0272: Node's modules, as @types/node types them, are the JS a person
// writes in Node: an import of `fs`'s, and `process`, a global, as
// react.dev's errors page reads its Markdown in `getStaticProps`.
test("node's modules are imported, and its globals called", async () => {
  const dir = fixture("node");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::{BufferEncoding, fs, process};

pub fn read(path: &str) -> String {
    match fs::read_file_sync(path, BufferEncoding::Utf8) {
        Ok(text) => text,
        Err(_) => "missing".to_string(),
    }
}

pub fn here() -> String {
    process::cwd()
}

pub fn twice(path: &str) -> String {
    let text = match fs::read_file_sync(path, BufferEncoding::Utf8) {
        Ok(text) => text,
        Err(_) => "missing".to_string(),
    };
    format!("{text}{text}")
}

pub fn read_or(path: &str, mut other: String) -> String {
    if let Ok(text) = fs::read_file_sync(path, BufferEncoding::Utf8) {
        other = text;
    }
    other
}

pub fn shouted(path: &str) -> String {
    match fs::read_file_sync(path, BufferEncoding::Utf8) {
        Ok(text) => format!("{text}!"),
        Err(_) => "missing".to_string(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('import { readFileSync } from "fs";');
  // A `match` of what it threw is JS's `try`, as the page has it (ADR 0035).
  expect(js).toContain('export function read(path) {\n  try {\n    return readFileSync(path, "utf8");\n  } catch {\n    return "missing";\n  }\n}');
  expect(js).toContain("return process.cwd();");
  // One whose value is a variable's too, as react.dev's errors page reads
  // its `.md` or the generic one, not a conditional of `$try` (ADR 0309).
  expect(js).toContain('  let text;\n  try {\n    text = readFileSync(path, "utf8");\n  } catch {\n    text = "missing";\n  }');
  writeFileSync(join(dir, "note.md"), "# Note");
  const { read, here } = await import(join(dir, "mir.js"));
  expect([read(join(dir, "note.md")), read(join(dir, "none.md")), here()]).toEqual(["# Note", "missing", process.cwd()]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.twice(join(dir, "note.md")), lib.twice(join(dir, "none.md"))]).toEqual(["# Note# Note", "missingmissing"]);
  expect([lib.readOr(join(dir, "note.md"), "x"), lib.readOr(join(dir, "none.md"), "x")]).toEqual(["# Note", "x"]);
  expect([lib.shouted(join(dir, "note.md")), lib.shouted(join(dir, "none.md"))]).toEqual(["# Note!", "missing"]);
});

// A callback-taking function made one that gives a promise, `util.promisify`,
// of `fs.readdir` and `fs.stat`, as react.dev's page route finds its Markdown
// in `getStaticPaths`: `const readdir = promisify(fs.readdir)`.
test("node's callback functions are promisified", async () => {
  const dir = fixture("node-promisify");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::{fs, path, util};

pub async fn listed(dir: String) -> Vec<String> {
    let readdir = util::promisify(fs::readdir);
    let stat = util::promisify(fs::stat);
    let mut kinds = Vec::new();
    for subdir in readdir(&dir).await {
        let res = path::resolve((&dir, &subdir));
        kinds.push(format!("{subdir}:{}", stat(&res).await.is_directory()));
    }
    kinds
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('import { promisify } from "util";');
  expect(js).toContain("promisify(readdir)");
  expect(js).toContain("promisify(stat)");
  mkdirSync(join(dir, "tree", "sub"), { recursive: true });
  writeFileSync(join(dir, "tree", "a.md"), "a");
  const lib = await import(join(dir, "mir.js"));
  expect((await lib.listed(join(dir, "tree"))).sort()).toEqual(["a.md:false", "sub:true"]);
  await expect(lib.listed(join(dir, "none"))).rejects.toThrow("ENOENT");
});

// `promisify`'s forms, as rustc checks a crate that uses node's: of none to
// five arguments, and of a borrowed first one, which any borrow is given,
// one a loop makes each time round. Without a warning: its impls of a
// borrowed first argument and of any first argument were told apart only
// by rustc's leak check, which warned that they conflict, and will refuse
// them (rust-lang/rust#56105).
test("promisify's forms are told apart without rustc's leak check", () => {
  const dir = fixture("node-promisify-forms");
  mkdirSync(join(dir, "src"), { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), `[package]
name = "forms"
version = "0.0.0"
edition = "2024"

[dependencies]
node = { package = "rust-js-node", path = ${JSON.stringify(join(root, "node"))} }
js = { package = "rust-js-builtins", path = ${JSON.stringify(join(root, "builtins"))} }

[workspace]
`);
  writeFileSync(join(dir, "src", "lib.rs"), `use js::{JsError, Promise};
use node::{fs, util};

type Done<T> = Box<dyn FnOnce(Option<&'static JsError>, T)>;

pub fn none(f: fn(Done<u32>)) -> Promise<u32> {
    util::promisify(f)()
}

pub fn one(f: fn(u32, Done<String>)) -> Promise<String> {
    util::promisify(f)(1)
}

pub fn unit(f: fn(u32, Box<dyn FnOnce(Option<&'static JsError>)>)) -> Promise<()> {
    util::promisify(f)(1)
}

pub fn two(f: fn(u32, String, Done<bool>)) -> Promise<bool> {
    util::promisify(f)(1, String::new())
}

pub fn borrowed(dirs: &[String]) -> Vec<Promise<Vec<String>>> {
    let readdir = util::promisify(fs::readdir);
    let mut listed = Vec::new();
    for dir in dirs {
        let path = format!("{dir}/");
        listed.push(readdir(&path));
    }
    listed
}
`);
  const check = runSync(["cargo", "check", "--offline", "--message-format=short", "--manifest-path", join(dir, "Cargo.toml"), "--target-dir", join(dir, "target")], dir, 300_000, { RUSTC_BOOTSTRAP: undefined });
  expect(check.stderr.split("\n").filter(line => /\b(?:warning|error)\b/.test(line) && !/generated \d+ warnings?/.test(line))).toEqual([]);
  expect(check.code).toBe(0);
});

// ADR 0272: `path`, each of @types/node's: its functions, its constants,
// and a platform's own, `win32.join`, as Node's `path` has them.
test("node's path module is bound whole", async () => {
  const dir = fixture("node-path");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::path::{self, FormatInputPathObject, ParsedPath, PlatformPath};

fn parts(parsed: ParsedPath) -> String {
    format!("{}|{}|{}|{}|{}", parsed.root, parsed.dir, parsed.base, parsed.ext, parsed.name)
}

pub fn all() -> Vec<String> {
    vec![
        path::normalize("/a//b/../c"),
        path::join(("a", "b", "../c")),
        path::resolve(("/a", "b")),
        path::relative("/a/b", "/a/c"),
        path::dirname("/a/b.txt"),
        path::basename("/a/b.txt", None),
        path::basename("/a/b.txt", Some(".txt")),
        path::extname("x.md"),
        path::sep.to_string(),
        path::delimiter.to_string(),
        parts(path::parse("/home/u/file.txt")),
        path::format(FormatInputPathObject { dir: Some("/x"), base: Some("y.js"), ..Default::default() }),
        path::to_namespaced_path("/a"),
        format!("{} {}", path::is_absolute("/a"), path::matches_glob("a/b.js", "a/*.js")),
    ]
}

pub fn platform(platform: &PlatformPath) -> Vec<String> {
    vec![
        platform.join(("a", "b")),
        platform.sep(),
        platform.delimiter(),
        platform.basename("c:/x/y.txt", Some(".txt")),
        parts(platform.parse("/a/b.c")),
        platform.posix().sep(),
    ]
}

pub fn platforms() -> Vec<Vec<String>> {
    vec![platform(path::posix), platform(path::win32)]
}

pub fn spread(parts: &[&str]) -> String {
    path::join(parts)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('join("a", "b", "../c")');
  // A `None` of an optional parameter is left out, a `Some` given (ADR 0443).
  expect(js).toContain('basename("/a/b.txt"),\n    basename("/a/b.txt", ".txt"),');
  expect(js).toContain("return join(...parts);");
  expect(js).toContain('platform.join("a", "b")');
  const lib = await import(join(dir, "lib.js"));
  const path = await import("node:path");
  const parts = (p: { root: string; dir: string; base: string; ext: string; name: string }) => [p.root, p.dir, p.base, p.ext, p.name].join("|");
  expect(lib.all()).toEqual([
    path.normalize("/a//b/../c"),
    path.join("a", "b", "../c"),
    path.resolve("/a", "b"),
    path.relative("/a/b", "/a/c"),
    path.dirname("/a/b.txt"),
    path.basename("/a/b.txt"),
    "b",
    ".md",
    path.sep,
    path.delimiter,
    "/|/home/u|file.txt|.txt|file",
    "/x/y.js",
    path.toNamespacedPath("/a"),
    "true true",
  ]);
  const platform = (p: typeof path.posix) => [p.join("a", "b"), p.sep, p.delimiter, p.basename("c:/x/y.txt", ".txt"), parts(p.parse("/a/b.c")), "/"];
  expect(lib.platforms()).toEqual([platform(path.posix), platform(path.win32)]);
  expect(lib.spread(["x", "y"])).toBe(path.join("x", "y"));
});

// ADR 0272: `buffer`, a `Buffer` made, read, written and searched, its
// numbers each of their Rust type, run by Node itself.
test("node's buffers are made, read and written", () => {
  const dir = fixture("node-buffer");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::BufferEncoding;
use node::buffer::{self, Buffer, TranscodeEncoding};

pub fn all() -> Vec<String> {
    let hi = Buffer::from_with_str("héllo", None);
    let hex = Buffer::from_with_str("ff00ab", Some(BufferEncoding::Hex));
    let bytes = Buffer::from(&[1, 2, 3]);
    let zeros = Buffer::alloc(4, None, None);
    zeros.write_uint16_be(0xbeef, Some(1));
    let both = Buffer::concat(&[&bytes, &hex], None);
    let numbers = Buffer::alloc(20, None, None);
    numbers.write_int32_le(-5, None);
    numbers.write_double_be(1.5, Some(4));
    numbers.write_big_uint64_le(7, Some(12));
    let filled = Buffer::alloc(5, Some(&"ab"), None);
    filled.fill(0, Some(3), None, None);
    vec![
        hi.to_string(None, None, None),
        hi.length().to_string(),
        Buffer::byte_length("héllo", None).to_string(),
        hex.to_string(Some(BufferEncoding::Base64), None, None),
        both.to_string(Some(BufferEncoding::Hex), None, None),
        zeros.to_string(Some(BufferEncoding::Hex), None, None),
        format!("{} {} {}", numbers.read_int32_le(None), numbers.read_double_be(Some(4)), numbers.read_big_uint64_le(Some(12))),
        format!("{} {}", bytes.read_uint8(Some(2)), bytes.read_uint_be(0, 3)),
        filled.to_string(None, None, None),
        format!("{} {} {}", hi.index_of("l", None, None), hi.last_index_of("l", None, None), hi.includes("xyz", None, None)),
        format!("{} {}", Buffer::compare_buffers(&bytes, &hex), bytes.equals(&Buffer::of((1, 2, 3)))),
        hi.subarray(Some(0), Some(1)).to_string(None, None, None),
        bytes.to_json().data.len().to_string(),
        format!("{} {}", Buffer::is_buffer(hi), Buffer::is_encoding("utf8")),
        format!("{} {}", buffer::is_utf8(hi), buffer::is_ascii(hi)),
        buffer::transcode(&Buffer::from_with_str("€", None), TranscodeEncoding::Utf8, TranscodeEncoding::Utf16le).length().to_string(),
        buffer::INSPECT_MAX_BYTES.to_string(),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  // Each optional argument a `None` is left out, as a person writes the
  // call; a union's `Some(&value)` is the value (ADR 0443).
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const call of ['Buffer.from("héllo")', 'Buffer.alloc(4)', 'Buffer.alloc(5, "ab")', "filled.fill(0, 3)", "numbers.writeDoubleBE(1.5, 4)", "numbers.readInt32LE()", 'hex.toString("base64")', "hi.subarray(0, 1).toString()"]) {
    expect(js).toContain(call);
  }
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { all } from "./lib.js";
console.log(JSON.stringify(all()));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual([
    "héllo", "6", "6", Buffer.from("ff00ab", "hex").toString("base64"), "010203ff00ab", "00beef00", "-5 1.5 7", "3 66051", "ababa".slice(0, 3) + "\u0000\u0000",
    "3 4 false", "-1 true", "h", "3", "true true", "true false", "2", "50",
  ]);
});

// ADR 0361: an emitter's events, each a name of its own and its listener's
// type: a program's own on an `EventEmitter`, and `process`'s, `exit` and a
// signal, run by Node itself.
test("node's events are typed by their emitters", () => {
  const dir = fixture("node-events");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::event::{Exit, UncaughtException};
use node::events::{self, Emits, EventEmitter, EventEmitterExt};
use node::js::JsError;
use node::process::{self, Signals};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.log")]
#[allow(unused_variables)]
fn log(what: &str) {
    unreachable!()
}

#[rust_js::name = "ping"]
pub struct Ping;

impl Emits<Ping> for EventEmitter {
    type Listener = dyn Fn(f64, &str);
    type Args = (f64, &'static str);
}

pub fn emitted() -> Vec<String> {
    let emitter = events::event_emitter::new(None);
    emitter.on(Ping, Box::new(|n, s| log(&format!("on {n} {s}"))));
    emitter.once(Ping, Box::new(|n, _| log(&format!("once {n}"))));
    let quiet: &'static dyn Fn(f64, &str) = events::listener(Box::new(|_, _| log("removed")));
    emitter.prepend_listener(Ping, Box::new(quiet));
    emitter.off(Ping, quiet);
    let first = emitter.emit(Ping, (1.0, "a"));
    let second = emitter.emit(Ping, (2.0, "b"));
    emitter.set_max_listeners(5.0);
    events::set_default_max_listeners(11.0);
    vec![
        events::default_max_listeners().to_string(),
        first.to_string(),
        second.to_string(),
        emitter.listener_count(Ping, None).to_string(),
        emitter.get_max_listeners().to_string(),
        emitter.event_names().join(","),
        events::get_max_listeners(emitter).to_string(),
    ]
}

pub async fn awaited() -> String {
    let emitter = events::event_emitter::new(None);
    let later = events::once(emitter, Ping, None);
    emitter.emit(Ping, (3.0, "c"));
    let (n, s) = later.await;
    format!("awaited {n} {s}")
}

pub fn signalled() {
    process::on(Exit, Box::new(|code| println!("exit {code}")));
    process::once(Signals::Sigusr2, Box::new(|signal| log(&format!("signal {signal:?}"))));
    process::emit(Signals::Sigusr2, (Signals::Sigusr2,));
}

// What @types/node gives \`emit\`, the error alone (ADR 0656).
pub fn uncaught(error: &'static JsError) -> bool {
    process::once(UncaughtException, Box::new(|error, _| log(&format!("uncaught {}", error.message()))));
    process::emit(UncaughtException, (error,))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('emitter.emit("ping", 1, "a")');
  // What a `None` leaves out isn't written (ADR 0443).
  for (const call of ["new EventEmitter()", 'emitter.listenerCount("ping")', 'once(emitter, "ping")']) {
    expect(js).toContain(call);
  }
  expect(js).toContain('process.on("exit", (code) =>');
  expect(js).toContain("EventEmitter.defaultMaxListeners = 11;");
  expect(js).toContain('return process.emit("uncaughtException", error);');
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { awaited, emitted, signalled, uncaught } from "./lib.js";
const logged = [];
globalThis.log = (what) => logged.push(what);
const got = emitted();
logged.push(await awaited());
signalled();
logged.push(uncaught(new Error("boom")));
console.log(JSON.stringify({ got, logged }));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  const [json, exit] = p.stdout.toString().trim().split("\n");
  expect(JSON.parse(json)).toEqual({
    got: ["11", "true", "true", "1", "5", "ping", "5"],
    logged: ["on 1 a", "once 1", "on 2 b", "awaited 3 c", "signal Sigusr2", "uncaught boom", true],
  });
  expect(exit).toBe("exit 0");
});

// ADR 0272: `process`, the global, read and written, run by Node itself,
// whose own `process` says what each should be.
test("node's process global is read and written", () => {
  const dir = fixture("node-process");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::js::dict;
use node::process::{self, NumberOrStr, ProcessFeaturesTypeScript};

pub fn all() -> Vec<String> {
    let here = process::cwd();
    let _ = process::chdir("/");
    let root = process::cwd();
    let _ = process::chdir(&here);
    dict::set(process::env(), "RUST_JS_TEST", "set".to_string());
    let since = process::cpu_usage(Some(process::cpu_usage(None)));
    let time = process::hrtime(Some(process::hrtime(None)));
    process::set_exit_code(3.0);
    let code = match process::exit_code() {
        Some(NumberOrStr::Number(n)) => n.to_string(),
        _ => "?".to_string(),
    };
    process::set_exit_code(0.0);
    process::set_trace_process_warnings(true);
    process::next_tick(Box::new(|| println!("tick")));
    println!("now");
    vec![
        process::argv().len().to_string(),
        format!("{:?}", process::platform()),
        format!("{:?}", process::arch()),
        root,
        (process::cwd() == here).to_string(),
        dict::get(process::env(), "RUST_JS_TEST").cloned().unwrap_or_default(),
        process::exec_path(),
        process::version(),
        dict::get(process::versions(), "node").cloned().unwrap_or_default(),
        (process::pid() > 0.0 && process::ppid() > 0.0 && process::uptime() > 0.0).to_string(),
        (time.0 >= 0.0 && process::hrtime_bigint() > 0).to_string(),
        (process::memory_usage().heap_used > 0.0 && process::memory_usage_rss() > 0.0).to_string(),
        (since.user >= 0.0 && process::resource_usage().max_rss > 0.0).to_string(),
        process::release().name,
        match process::features().typescript {
            ProcessFeaturesTypeScript::Strip => "strip".to_string(),
            ProcessFeaturesTypeScript::Transform => "transform".to_string(),
            ProcessFeaturesTypeScript::None(_) => "false".to_string(),
        },
        process::config().variables.host_arch,
        code,
        process::kill(process::pid(), Some(&0.0)).map(|sent| sent.to_string()).unwrap_or_default(),
        format!("{} {}", process::get_builtin_module("fs").is_some(), process::get_builtin_module("nope").is_some()),
        process::allowed_node_environment_flags().contains("--max-old-space-size").to_string(),
        process::trace_process_warnings().to_string(),
        process::has_uncaught_exception_capture_callback().to_string(),
        process::getuid().to_string(),
        process::argv0(),
        process::exec_argv().len().to_string(),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { all } from "./lib.js";
const cap = (s) => s[0].toUpperCase() + s.slice(1);
const got = all();
const want = [
  String(process.argv.length), cap(process.platform), cap(process.arch), "/", "true", "set", process.execPath, process.version,
  process.versions.node, "true", "true", "true", "true", process.release.name, String(process.features.typescript),
  process.config.variables.host_arch, "3", "true", "true false", "true", "true", "false", String(process.getuid()), process.argv0,
  String(process.execArgv.length),
];
console.log(JSON.stringify({ got, want }));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  const out = p.stdout.toString();
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  const lines = out.trim().split("\n");
  // `nextTick`'s callback runs once the script's own code is done.
  expect([lines[0], lines[2]]).toEqual(["now", "tick"]);
  const { got, want } = JSON.parse(lines[1]);
  expect(got).toEqual(want);
});

// ADR 0272: `querystring`, a query's text from a dictionary of values and
// back, each value one text or a list of them.
test("node's querystring module is bound whole", async () => {
  const dir = fixture("node-querystring");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::js::dict;
use node::querystring::{
    self, ParseOptions, ParsedUrlQuery, ParsedUrlQueryInput, ParsedUrlQueryInputItem, ParsedUrlQueryInputValue,
    ParsedUrlQueryValue, StringifyOptions,
};

fn shown(query: &ParsedUrlQuery) -> String {
    dict::entries(query)
        .into_iter()
        .map(|(key, value)| match value {
            ParsedUrlQueryValue::One(one) => format!("{key}={one}"),
            ParsedUrlQueryValue::Many(many) => format!("{key}=[{}]", many.join(",")),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn all() -> Vec<String> {
    let input: &ParsedUrlQueryInput = dict::new();
    dict::set(input, "name", Some(ParsedUrlQueryInputValue::Str("a b".to_string())));
    dict::set(input, "n", Some(ParsedUrlQueryInputValue::Number(2.0)));
    dict::set(input, "ok", Some(ParsedUrlQueryInputValue::Bool(true)));
    dict::set(input, "big", Some(ParsedUrlQueryInputValue::BigInt(7)));
    dict::set(input, "tags", Some(ParsedUrlQueryInputValue::List(vec![ParsedUrlQueryInputItem::Str("x".to_string()), ParsedUrlQueryInputItem::Number(1.0)])));
    dict::set(input, "none", None);
    vec![
        querystring::stringify(Some(input), None, None, None),
        querystring::stringify(Some(input), Some(";"), None, None),
        querystring::stringify(Some(input), Some(";"), Some(":"), None),
        querystring::stringify(Some(input), Some("&"), Some("="), Some(StringifyOptions { encode_uri_component: Some(Box::new(|s: &str| s.to_uppercase())) })),
        querystring::encode(Some(input), None, None, None),
        shown(querystring::parse("a=1&b=2&a=3", None, None, None)),
        shown(querystring::parse("a=1;b=2", Some(";"), None, None)),
        shown(querystring::parse("a:1;b:2", Some(";"), Some(":"), None)),
        shown(querystring::parse("a=1&b=2&c=3", Some("&"), Some("="), Some(ParseOptions { max_keys: Some(2.0), ..Default::default() }))),
        shown(querystring::decode("x=%20y", None, None, None)),
        querystring::escape("a b&c"),
        querystring::unescape("a%20b"),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('from "querystring";');
  expect(js).toContain('stringify(input),\n    stringify(input, ";"),\n    stringify(input, ";", ":"),');
  const lib = await import(join(dir, "lib.js"));
  const qs = await import("node:querystring");
  const input = { name: "a b", n: 2, ok: true, big: 7n, tags: ["x", 1], none: undefined };
  expect(lib.all()).toEqual([
    qs.stringify(input),
    qs.stringify(input, ";"),
    qs.stringify(input, ";", ":"),
    qs.stringify(input, "&", "=", { encodeURIComponent: (s: string) => s.toUpperCase() }),
    qs.stringify(input),
    "a=[1,3] b=2",
    "a=1 b=2",
    "a=1 b=2",
    "a=1 b=2",
    "x= y",
    qs.escape("a b&c"),
    "a b",
  ]);
});

// ADR 0272: `url`, its WHATWG classes webapi's, as Node's are the globals,
// and its own: file URLs, domains, and a URL written without its parts.
test("node's url module binds its own and the WHATWG classes", async () => {
  const dir = fixture("node-url");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::url::{self, FileUrlToPathOptions, PathToFileUrlOptions, URLFormatOptions, URLSearchParams};
use node::webapi;

pub fn all() -> Vec<String> {
    let file = url::path_to_file_url("/tmp/a b.txt", None);
    let link = webapi::url::new("https://user:pw@example.com/p?q=1#top", None);
    let params: &URLSearchParams = link.search_params();
    vec![
        file.href(),
        url::file_url_to_path("file:///tmp/a%20b.txt", None),
        url::file_url_to_path(file, None),
        url::file_url_to_path("file:///C:/x", Some(FileUrlToPathOptions { windows: Some(true) })),
        url::path_to_file_url("C:\\\\x y", Some(PathToFileUrlOptions { windows: Some(true) })).href(),
        url::domain_to_ascii("español.com"),
        url::domain_to_unicode("xn--espaol-zwa.com"),
        url::resolve("/one/two", "three"),
        url::format(link, None),
        url::format(link, Some(URLFormatOptions { auth: Some(false), fragment: Some(false), ..Default::default() })),
        params.get("q").unwrap_or_default(),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('from "url";');
  const lib = await import(join(dir, "lib.js"));
  const url = await import("node:url");
  const link = new URL("https://user:pw@example.com/p?q=1#top");
  expect(lib.all()).toEqual([
    url.pathToFileURL("/tmp/a b.txt").href,
    "/tmp/a b.txt",
    "/tmp/a b.txt",
    url.fileURLToPath("file:///C:/x", { windows: true }),
    url.pathToFileURL("C:\\x y", { windows: true }).href,
    url.domainToASCII("español.com"),
    url.domainToUnicode("xn--espaol-zwa.com"),
    url.resolve("/one/two", "three"),
    url.format(link),
    url.format(link, { auth: false, fragment: false }),
    "1",
  ]);
});

// ADR 0272: `url`'s legacy `parse` and `format`, a URL's parts, `null`
// where it hasn't one, and its query a text or parsed.
test("node's url module parses and formats legacy URL objects", async () => {
  const dir = fixture("node-url-legacy");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::js::dict;
use node::querystring::{ParsedUrlQueryInput, ParsedUrlQueryInputValue, ParsedUrlQueryValue};
use node::url::{self, Url, UrlObject, UrlObjectPort, UrlObjectQuery, UrlQuery, UrlWithStringQuery};

fn or(part: Option<String>) -> String {
    part.unwrap_or_else(|| "-".to_string())
}

pub fn parsed() -> Vec<String> {
    let link: UrlWithStringQuery = url::parse("https://u:p@example.com:8080/a/b?x=1&x=2#h");
    let bare: UrlWithStringQuery = url::parse("/only/path");
    let query: Url = url::parse_with_parse_query_string("https://example.com/?x=1&x=2&y=3", true, None);
    let host: Url = url::parse_with_parse_query_string("//example.com/p", false, Some(true));
    let parsed = match query.query {
        Some(UrlQuery::Parsed(parsed)) => match dict::get(parsed, "x") {
            Some(ParsedUrlQueryValue::Many(many)) => many.join(","),
            _ => "?".to_string(),
        },
        _ => "?".to_string(),
    };
    vec![
        link.href,
        or(link.protocol),
        or(link.auth),
        or(link.host),
        or(link.port),
        or(link.hostname),
        or(link.hash),
        or(link.search),
        or(link.query),
        or(link.pathname),
        or(link.path),
        format!("{:?}", link.slashes),
        or(bare.host),
        or(bare.query),
        parsed,
        or(host.host),
    ]
}

pub fn formatted() -> Vec<String> {
    let query: &ParsedUrlQueryInput = dict::new();
    dict::set(query, "q", Some(ParsedUrlQueryInputValue::Str("a b".to_string())));
    vec![
        url::format_with_url_object(UrlObject {
            protocol: Some("https"),
            hostname: Some("example.com"),
            port: Some(UrlObjectPort::Number(8080.0)),
            pathname: Some("/x"),
            query: Some(UrlObjectQuery::Input(query)),
            ..Default::default()
        }),
        url::format_with_url_object(UrlObject { host: Some("example.com"), query: Some(UrlObjectQuery::Str("a=1")), ..Default::default() }),
        url::format_with_str("https://example.com/a?b#c"),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const lib = await import(join(dir, "lib.js"));
  const url = await import("node:url");
  const link = url.parse("https://u:p@example.com:8080/a/b?x=1&x=2#h");
  const or = (part: string | null) => part ?? "-";
  expect(lib.parsed()).toEqual([
    link.href, or(link.protocol), or(link.auth), or(link.host), or(link.port), or(link.hostname), or(link.hash), or(link.search),
    or(link.query), or(link.pathname), or(link.path), "Some(true)", "-", "-", "1,2", "example.com",
  ]);
  expect(lib.formatted()).toEqual([
    url.format({ protocol: "https", hostname: "example.com", port: 8080, pathname: "/x", query: { q: "a b" } }),
    url.format({ host: "example.com", query: "a=1" }),
    url.format("https://example.com/a?b#c"),
  ]);
});

// Node's http request and response, as a server's handler is given them,
// and Next.js's Pages Router: what's read of one and written to the other.
test("node's http request is read and its response written", async () => {
  const dir = fixture("node-http");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::http::{IncomingMessage, ServerResponse};

pub fn handle(req: &IncomingMessage, res: &ServerResponse) {
    res.set_status_code(201.0);
    res.set_header("x-method", req.method().unwrap_or_default().as_str());
    res.set_header("x-many", &["a", "b"][..]);
    let seen = req.has_header_named("x-seen");
    res.end(Some(&format!("{} {} {}", req.url().unwrap_or_default(), req.http_version(), seen)));
}
`.replace("let seen = req.has_header_named(\"x-seen\");", "let seen = req.raw_headers().iter().any(|h| h == \"x-seen\");"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("res.statusCode = 201;");
  expect(js).toContain('res.setHeader("x-many", ["a", "b"]);');
  const { handle } = await import(join(dir, "lib.js"));
  const { createServer } = await import("node:http");
  const server = createServer(handle);
  await new Promise<void>((done) => server.listen(0, done));
  try {
    const { port } = server.address() as { port: number };
    const response = await fetch(`http://localhost:${port}/search?q=rust`, { headers: { "x-seen": "1" } });
    expect([response.status, response.headers.get("x-method"), response.headers.get("x-many"), await response.text()]).toEqual([201, "GET", "a, b", "/search?q=rust 1.1 true"]);
  } finally {
    server.close();
  }
});

// ADR 0648: V8's stack trace API, as @types/node types it, in the forms
// React's owner stacks write it: `formatOwnerStack` saves the hook, sets
// `DefaultPrepareStackTrace`, `undefined` in a browser's build, reads the
// error's stack and restores it; `DefaultPrepareStackTraceV8` is a hook of
// its own, the module's default, which a server's build sets. Run in Node.
test("V8's stack trace hook is saved, set and restored", () => {
  const dir = fixture("node-stack-trace");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;
use node::{CallSite, js_error};

pub mod default_prepare_stack_trace {
    use js::JsError;
    use node::CallSite;

    js::export_default! {
        pub const DEFAULT_PREPARE_STACK_TRACE: Option<&'static dyn Fn(&JsError, &[&CallSite]) -> String> = None;
    }
}

pub mod default_prepare_stack_trace_v8 {
    use js::JsError;
    use node::CallSite;

    fn prepare_stack_trace(error: &JsError, structured_stack_trace: &[&CallSite]) -> String {
        let mut stack = error.name() + ": " + &error.message();
        for call_site in structured_stack_trace {
            stack += "\\n    at ";
            stack += &call_site.to_string();
        }
        stack
    }

    js::export_default!(prepare_stack_trace);

    pub fn install() {
        node::js_error::set_prepare_stack_trace(Some(&prepare_stack_trace));
    }
}

use default_prepare_stack_trace::DEFAULT_PREPARE_STACK_TRACE;

pub fn format_owner_stack(error: &JsError) -> Option<String> {
    let prev_prepare_stack_trace = js_error::prepare_stack_trace();
    js_error::set_prepare_stack_trace(DEFAULT_PREPARE_STACK_TRACE);
    let stack = error.stack();
    js_error::set_prepare_stack_trace(prev_prepare_stack_trace);
    stack
}

fn files(_: &JsError, structured_stack_trace: &[&CallSite]) -> String {
    let first = structured_stack_trace.first();
    let file = first.and_then(|call_site| call_site.get_file_name()).unwrap_or_default();
    let line = first.and_then(|call_site| call_site.get_line_number()).unwrap_or_default();
    format!("{file}:{line}")
}

pub fn files_of(error: &JsError) -> Option<String> {
    js_error::set_prepare_stack_trace(Some(&files));
    let stack = error.stack();
    js_error::set_prepare_stack_trace::<String>(None);
    stack
}

pub fn limited(limit: f64) -> f64 {
    let previous = js_error::stack_trace_limit();
    js_error::set_stack_trace_limit(limit);
    previous
}

pub fn captured(target: &JsError) {
    js_error::capture_stack_trace(target, None);
}

pub fn trimmed(target: &JsError, above: &'static dyn Fn()) {
    js_error::capture_stack_trace(target, Some(above));
}

fn function_of(_: &JsError, structured_stack_trace: &[&CallSite]) -> bool {
    structured_stack_trace.first().and_then(|call_site| call_site.get_function()).is_some()
}

pub fn has_function(error: &JsError) -> bool {
    js_error::set_prepare_stack_trace(Some(&function_of));
    let stack = error.stack();
    js_error::set_prepare_stack_trace::<bool>(None);
    stack.is_some()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  // Each as React writes it: the hook a property of `Error`, read and
  // written, `undefined` too; the default a value of its own.
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function formatOwnerStack(error) {
  const prevPrepareStackTrace = Error.prepareStackTrace;
  Error.prepareStackTrace = DEFAULT_PREPARE_STACK_TRACE;
  const stack = error.stack;
  Error.prepareStackTrace = prevPrepareStackTrace;
  return stack;
}`);
  expect(js).toContain("  Error.prepareStackTrace = files;\n  const stack = error.stack;\n  Error.prepareStackTrace = undefined;\n");
  expect(js).toContain("  const previous = Error.stackTraceLimit;\n  Error.stackTraceLimit = limit;\n");
  expect(js).toContain("  Error.captureStackTrace(target);\n");
  expect(js).toContain("  Error.captureStackTrace(target, above);\n");
  expect(js).toContain(".getFunction()");
  expect(readFileSync(join(dir, "default_prepare_stack_trace.js"), "utf8")).toContain("\nexport default undefined;\n");
  const v8 = readFileSync(join(dir, "default_prepare_stack_trace_v8.js"), "utf8");
  expect(v8).toContain('    stack += "\\n    at " + callSite.toString();\n');
  expect(v8).toContain("export function install() {\n  Error.prepareStackTrace = prepareStackTrace;\n}");
  expect(v8).toContain("export default prepareStackTrace;");
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { captured, filesOf, formatOwnerStack, limited, trimmed } from "./lib.js";
import prepareStackTrace, { install } from "./default_prepare_stack_trace_v8.js";

const ours = () => "ours";
Error.prepareStackTrace = ours;
const owner = formatOwnerStack(new Error("react-stack-top-frame")).split("\\n");
const restored = Error.prepareStackTrace === ours;
install();
const installed = Error.prepareStackTrace === prepareStackTrace;
const v8 = new Error("boom").stack.split("\\n");
const files = filesOf(new Error("here"));
const cleared = Error.prepareStackTrace === undefined;
const previous = limited(1);
const one = new Error("one").stack.split("\\n").length;
limited(previous);
const target = { name: "Target", message: "here" };
captured(target);
function outer() {
  const inner = {};
  trimmed(inner, outer);
  return inner;
}
const around = () => outer();
const trimmedStack = around().stack.split("\\n");
console.log(JSON.stringify({
  owner: owner[0], at: owner[1].startsWith("    at "), restored, installed, v8: v8[0], v8At: v8[1].startsWith("    at "), files, cleared, previous, one,
  target: target.stack.split("\\n")[0], capturedAt: target.stack.includes("at captured"), trimmed: trimmedStack[1].includes("around"),
}));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual({
    owner: "Error: react-stack-top-frame",
    at: true,
    restored: true,
    installed: true,
    v8: "Error: boom",
    v8At: true,
    files: `file://${join(dir, "run.js")}:11`,
    cleared: true,
    previous: 10,
    one: 2,
    target: "Target: here",
    capturedAt: true,
    trimmed: true,
  });
});

// ADRs 0657 and 0658: React's reportGlobalError as its port writes it, the
// host's reportError or a polyfill: a const of a conditional, each branch's
// comment before its value, as upstream's; run in Node, whose reportError
// is taken out to run the polyfill.
test("React's reportGlobalError is a conditional with its comments", () => {
  const dir = fixture("report-global-error");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use js::{Kind, Unknown};
use webapi::{ErrorEventInit, console, error_event, window};

thread_local! {
    static REPORT_GLOBAL_ERROR: fn(Option<&'static Unknown>) = if js::is_defined(webapi::global::report_error::<Option<&Unknown>>) {
        // In modern browsers, reportError will dispatch an error event,
        // emulating an uncaught JavaScript error.
        webapi::global::report_error
    } else {
        |error| {
            if js::is_defined(window) && js::is_defined(error_event::new) {
                // Browser Polyfill
                let message = match error.and_then(|error| js::get(error, "message")).map(js::classify) {
                    Some(Kind::String(message)) => message.to_string(),
                    _ => js::string(error),
                };
                let event = error_event::new(
                    "error",
                    Some(ErrorEventInit {
                        bubbles: Some(true),
                        cancelable: Some(true),
                        message: Some(&message),
                        error: error.map(|error| error as &dyn js::IntoUnknown),
                        ..Default::default()
                    }),
                );
                let should_log = window.dispatch_event(event);
                if !should_log {
                    return;
                }
            } else if js::is_defined(node::process::emit::<node::event::UncaughtException>) {
                // Node Polyfill
                node::process::emit(node::event::UncaughtException, (unsafe { js::cast(error) },));
                return;
            }
            console::error((error,));
        }
    };
}

js::export_default!(REPORT_GLOBAL_ERROR);
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "--extern", `js=${join(node, "libjs.rmeta")}`, "--extern", `webapi=${join(node, "libwebapi.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`const reportGlobalError =
  typeof reportError !== "undefined"
    ? // In modern browsers, reportError will dispatch an error event,
      // emulating an uncaught JavaScript error.
      reportError
    : (error) => {
`);
  expect(js).toContain("// Browser Polyfill\n");
  expect(js).toContain('// Node Polyfill\n          process.emit("uncaughtException", error);\n');
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `const seen = [];
const host = globalThis.reportError;
globalThis.reportError = undefined;
process.on("uncaughtException", (error) => seen.push("node " + error.message));
const { default: viaNode } = await import("./lib.js");
viaNode(new Error("boom"));
globalThis.reportError = (error) => seen.push("host " + error.message);
const { default: viaHost } = await import("./lib.js?again");
viaHost(new Error("bang"));
globalThis.reportError = host;
console.log(JSON.stringify(seen));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual(["node boom", "host bang"]);
});
