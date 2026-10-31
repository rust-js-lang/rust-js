// A binding's path is JS's (ADR 1033): the global it starts from, then each
// property by its name, `console.log`, or by a quoted key, `window['$RT']`,
// which a minifier keeps, as React's Fizz runtime writes the globals the
// server's inline scripts set.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const externs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("a quoted key of a binding's path is read and written by its key", async () => {
  const dir = fixture("quoted-path");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "window['$RT']"]
    safe static RT: Option<&'static js::Unknown>;
    #[link_name = "window['$RN']"]
    safe static RN: f64;
}

#[cfg_attr(rust_js, rust_js::link_name = "set window['$RT']")]
fn set_rt(time: f64) {
    let _ = time;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get window['$RT'].toFixed")]
fn rt_to_fixed() -> Option<&'static js::Unknown> {
    unreachable!()
}

pub fn mark(now: f64) -> bool {
    set_rt(now);
    js::is_number(RN) && js::is_defined(RT) && rt_to_fixed().is_some()
}

pub fn shell_time() -> Option<&'static js::Unknown> {
    RT
}

pub fn shadowed(window: f64) -> f64 {
    set_rt(window + 1.0);
    window
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('window["$RT"] = now;');
  expect(js).toContain('typeof window["$RN"] === "number"');
  expect(js).toContain('window["$RT"].toFixed != null');
  expect(js).toContain('return window["$RT"];');
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as any;
  g.window = { $RN: 1 };
  try {
    expect([lib.mark(5), lib.shellTime(), g.window.$RT]).toEqual([true, 5, 5]);
    expect([lib.shadowed(2), g.window.$RT]).toEqual([2, 3]);
  } finally {
    delete g.window;
  }
});

test("a binding's path that isn't JS's is refused, naming the quoted key", () => {
  const dir = fixture("refused-path");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "window[\\"$RT\\"]"]
    safe static RT: Option<&'static js::Unknown>;
    #[link_name = "window['a'b']"]
    safe static AB: Option<&'static js::Unknown>;
}

#[cfg_attr(rust_js, rust_js::link_name = "set window.$R-T")]
fn set_rt(time: f64) {
    let _ = time;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./shell.js#times.shell-time")]
fn shell_time() -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "['matchMedia']")]
fn match_media(this: &js::Unknown) {
    let _ = this;
    unreachable!()
}

pub fn mark(now: f64, on: &js::Unknown) -> bool {
    set_rt(now + shell_time());
    match_media(on);
    RT.is_some() && AB.is_some()
}
`);
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs], root, 60_000);
  expect(p.code).not.toBe(0);
  const out = p.stderr;
  expect(out).toContain("`window.$R-T` isn't a binding's path: `.$R-T` isn't a JS name's property: a key that isn't one is quoted, `['$R-T']`");
  expect(out).toContain("`window[\"$RT\"]` isn't a binding's path: `[\"$RT\"]` isn't a property: one is `.name` or a quoted key, `['name']`");
  expect(out).toContain("`window['a'b']` isn't a binding's path: `['a'b']` isn't a key: one of at least one character, without a quote or a backslash");
  expect(out).toContain("`./shell.js#times.shell-time` isn't a binding's path: `.shell-time` isn't a JS name's property");
  expect(out).toContain("`['matchMedia']` isn't a binding's path: a method's property is named");
});
