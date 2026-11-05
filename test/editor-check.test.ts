// The check an editor runs, `rust-js-check` as rust-analyzer's
// `check.overrideCommand` (ADR 0222): the app's `cargo check` through
// rust-js, which reads inside JSX, as a plain rustc doesn't.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, root } from "./support";

beforeAll(buildCompiler, 600_000);

/** An app of `source`, its `src/App.rs`, using the react and js crates. */
function app(name: string, source: string): string {
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
  writeFileSync(join(dir, "src", "App.rs"), source);
  writeFileSync(join(dir, "package.json"), JSON.stringify({ name: name, private: true }));
  return dir;
}

/** What `rust-js-check` says of the app at `dir`, given `args` too: Cargo's
 * JSON messages, each compiler message's level and text, and its exit code. */
function check(dir: string, ...args: string[]) {
  const p = Bun.spawnSync(["bun", join(root, "tooling", "check.js"), dir, ...args], {
    env: { ...process.env, RUST_JS_COMPILER: compiler },
    stderr: "pipe",
  });
  const messages = p.stdout.toString().split("\n").filter(Boolean).map((line) => JSON.parse(line))
    .filter((message) => message.reason === "compiler-message")
    .map((message) => message.message)
    .filter((message) => message.spans.length > 0);
  return { code: p.exitCode, messages, stderr: p.stderr.toString() };
}

// A variable and an import only JSX uses, and a component only
// `js::export_default!` exports, are used: nothing to warn of.
const greeting = `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct GreetingProps<'a> {
    pub name: &'a str,
}

fn Greeting(GreetingProps { name }: GreetingProps) -> JSX::Element {
    let label = format!("Hello, {name}");
    jsx! { <p title={label.as_str()}>{"Hi"}</p> }
}

js::export_default!(Greeting);
`;

test("the editor's check reads inside JSX, and an export_default! component is used", () => {
  const { code, messages, stderr } = check(app("editor-check", greeting));
  expect([code, messages.map((m) => `${m.level}: ${m.message}`)], stderr).toEqual([0, []]);
}, 600_000);

test("the editor's check says what's wrong inside JSX, where it is", () => {
  const broken = greeting.replace(`<p title={label.as_str()}>`, `<p title={label.as_str()} autoFocus={"yes"}>`);
  const { code, messages } = check(app("editor-check-error", broken));
  expect(code).not.toBe(0);
  const error = messages.find((m) => m.level === "error");
  expect(error?.message).toContain("mismatched types");
  expect(error?.spans[0].file_name).toEndWith("src/App.rs");
  expect(error?.spans[0].line_start).toBe(10);
}, 600_000);

/** A full-stack workspace: `client`, the page rust-js compiles, of
 * `source`, and `server`, native Rust that only a native target compiles,
 * its `main.rs` of `server`. */
function fullStack(name: string, server: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "Cargo.toml"), `[workspace]\nmembers = ["client", "server"]\nresolver = "3"\n`);
  writeFileSync(join(dir, "package.json"), JSON.stringify({ name: name, private: true }));
  mkdirSync(join(dir, "client", "src"), { recursive: true });
  writeFileSync(join(dir, "client", "Cargo.toml"), `[package]
name = "client"
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
  writeFileSync(join(dir, "client", "src", "App.rs"), greeting);
  mkdirSync(join(dir, "server", "src"), { recursive: true });
  writeFileSync(join(dir, "server", "Cargo.toml"), `[package]\nname = "server"\nversion = "0.1.0"\nedition = "2024"\n`);
  writeFileSync(join(dir, "server", "src", "main.rs"), server);
  return dir;
}

// Unix's file modes: std has them on a native target, not on rust-js's.
const server = `use std::os::unix::fs::PermissionsExt;

fn main() {
    let unused = 1;
    let mode = std::fs::metadata("Cargo.toml").unwrap().permissions().mode();
    println!("{mode:o}");
}
`;

test("the editor's check of a full-stack workspace reads its client through rust-js and its server natively", () => {
  const { code, messages, stderr } = check(fullStack("editor-check-full-stack", server), "--package", "client");
  // Every target's, as rust-analyzer's own check: the binary's and its
  // tests' say one warning twice, which rust-analyzer shows once.
  const said = new Set(messages.map((m) => `${m.level}: ${m.message} (${m.spans[0].file_name})`));
  expect([code, [...said]], stderr).toEqual([0, ["warning: unused variable: `unused` (server/src/main.rs)"]]);
}, 600_000);

test("the editor's check says a native package's error, and fails", () => {
  const broken = server.replace("println!(\"{mode:o}\");", "let wrong: bool = mode;");
  const { code, messages } = check(fullStack("editor-check-full-stack-error", broken), "--package", "client");
  expect(code).not.toBe(0);
  const error = messages.find((m) => m.level === "error");
  expect(error?.message).toContain("mismatched types");
  expect(error?.spans[0].file_name).toBe("server/src/main.rs");
}, 600_000);

test("the editor's check refuses what it doesn't take, saying what it does", () => {
  const p = Bun.spawnSync(["bun", join(root, "tooling", "check.js"), "--packages", "client"], { stderr: "pipe" });
  expect(p.exitCode).toBe(2);
  expect(p.stderr.toString()).toContain("usage: rust-js-check [<app-dir>] [--package <name>]...");
});
