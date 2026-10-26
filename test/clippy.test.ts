// `rust-js clippy` (ADR 0888): an app linted by clippy's lints as rust-js
// reads it: inside JSX, and with what JS reads used (ADR 0440), which a
// plain `cargo clippy` warns of as dead code. Only what clippy finds of the
// Rust itself is said.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, root } from "./support";

beforeAll(buildCompiler, 600_000);

/** An app of `files`, by their paths in `src/`, `App.rs` its crate's root,
 * using the react and js crates. */
function app(name: string, files: Record<string, string>): string {
  const dir = fixture(name);
  mkdirSync(join(dir, "src"), { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), `[package]
name = "app"
version = "0.1.0"
edition = "2024"

[lib]
path = "src/App.rs"

[dependencies]
js = { package = "rust-js-builtins", path = ${JSON.stringify(join(root, "builtins"))} }
react = { package = "rust-js-react", path = ${JSON.stringify(join(root, "react"))} }

[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ["cfg(rust_js)"] }
`);
  for (const [file, source] of Object.entries(files)) writeFileSync(join(dir, "src", file), source);
  writeFileSync(join(dir, "package.json"), JSON.stringify({ name, private: true }));
  return dir;
}

/** What `rust-js clippy -- ..lints` says of the app at `dir`: each
 * diagnostic's first line, and its exit code. */
function clippy(dir: string, ...lints: string[]) {
  const p = Bun.spawnSync(["bun", join(root, "tooling", "cli.js"), "clippy", "--", ...lints], {
    cwd: dir,
    env: { ...process.env, RUST_JS_COMPILER: compiler },
    stderr: "pipe",
  });
  const stderr = p.stderr.toString();
  const said = stderr.split("\n").filter((line) => /^(warning|error)(\[\w+\])?:/.test(line) && !/could not compile|warnings? emitted|generated \d+ warning/.test(line));
  return { code: p.exitCode, said, stderr };
}

// A variable and an import only JSX uses; a component and a function only
// `js::export_default!` names (ADR 0611), as React's
// DefaultPrepareStackTraceV8 has its hook; and a struct of descriptors only
// `Object.defineProperties` reads, React's ConsolePatchingDev's `Logs` (ADR
// 0666); and a class only `js::class::<T>()` hands JS. Each is used, as
// rust-js reads it. And an `if` in an `if`, which clippy's `collapsible_if`
// says is one.
const files = {
  "App.rs": `mod logs;
mod shim;
mod v8;

use react::{JSX, jsx};

pub struct GreetingProps<'a> {
    pub name: &'a str,
}

fn Greeting(GreetingProps { name }: GreetingProps) -> JSX::Element {
    let label = format!("Hello, {name}");
    jsx! { <p title={label.as_str()}>{"Hi"}</p> }
}

js::export_default!(Greeting);

pub fn both(a: bool, b: bool) -> u32 {
    if a {
        if b {
            return 1;
        }
    }
    0
}
`,
  "logs.rs": `use js::{PropertyDescriptor, PropertyDescriptorMap, Unknown, object};

struct Logs<'a> {
    info: PropertyDescriptor<'a>,
    log: PropertyDescriptor<'a>,
}

unsafe impl PropertyDescriptorMap for Logs<'_> {}

pub fn disable(console: &Unknown) {
    let props = PropertyDescriptor {
        configurable: Some(true),
        ..Default::default()
    };
    object::define_properties(
        console,
        &Logs {
            info: props,
            log: props,
        },
    );
}
`,
  "v8.rs": `fn prepare_stack_trace(name: &str) -> String {
    format!("{name}: ")
}

js::export_default!(prepare_stack_trace);
`,
  // A class only \`js::class::<T>()\` hands JS, as React's
  // ReactFiberCacheComponent has its unexported \`AbortControllerShim\`:
  // JS makes it, \`new AbortControllerShim()\`, and reads its fields and
  // calls its prototype's methods, so they, and what they use, are used
  // (ADR 0888, amended).
  "shim.rs": `use std::cell::Cell;

use js::Class;

struct Signal {
    aborted: Cell<bool>,
}

impl Signal {
    fn abort(&self) {
        self.aborted.set(true);
    }
}

fn unaborted() -> Signal {
    Signal {
        aborted: Cell::new(false),
    }
}

struct Reason(&'static str);

struct Options {
    capture: bool,
}

fn listening(options: &Options) -> u32 {
    u32::from(options.capture)
}

#[cfg_attr(rust_js, rust_js::class)]
struct AbortControllerShim {
    signal: Signal,
    reason: Reason,
    listeners: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl AbortControllerShim {
    fn new() -> Self {
        AbortControllerShim {
            signal: unaborted(),
            reason: Reason("none"),
            listeners: listening(&Options { capture: false }),
        }
    }

    fn abort(&self) {
        self.signal.abort();
    }
}

pub fn shim() -> &'static Class {
    js::class::<AbortControllerShim>()
}
`,
};

// Its lint flags are clippy's: a warning, then, with `-D warnings`, an
// error, the crate linted again as its flags changed.
test("rust-js clippy says what clippy finds, and nothing JS uses", () => {
  const dir = app("clippy-app", files);
  const warned = clippy(dir);
  expect([warned.code, warned.said], warned.stderr).toEqual([0, ["warning: this `if` statement can be collapsed"]]);
  const denied = clippy(dir, "-D", "warnings");
  expect([denied.code, denied.said], denied.stderr).toEqual([101, ["error: this `if` statement can be collapsed"]]);
}, 600_000);

// Without the `if` in an `if`, nothing, and no JS is written beside the Rust.
test("rust-js clippy of clean Rust says nothing", () => {
  const clean = { ...files, "App.rs": files["App.rs"].replace("    if a {\n        if b {\n            return 1;\n        }\n    }\n", "    if a && b {\n        return 1;\n    }\n") };
  const dir = app("clippy-clean", clean);
  const { code, said, stderr } = clippy(dir, "-D", "warnings");
  expect([code, said], stderr).toEqual([0, []]);
  expect(readdirSync(join(dir, "src")).sort()).toEqual(["App.rs", "logs.rs", "shim.rs", "v8.rs"]);
}, 600_000);
