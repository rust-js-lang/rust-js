// A program, and the crates it uses, are plain stable Rust (ADRs 0112,
// 0113): rust-js compiles the crates with its tool registered, and a plain
// rustc, a user's own `cargo check` and editor, checks them and the app.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, compiler, fixture, root } from "./support";

beforeAll(buildCompiler, 600_000);

// The crates a program depends on, js, webapi and react, are stable Rust too
// (ADR 0112): rust-js compiles them, `--rustc`, with its tool registered, so
// they need no `register_tool`, nor any feature, nor `RUSTC_BOOTSTRAP`.
test("the binding crates are stable Rust, which rust-js compiles", () => {
  for (const root of ["builtins/src/lib.rs", "webapi/src/lib.rs", "react/src/lib.rs"]) {
    const source = readFileSync(join(import.meta.dir, "..", root), "utf8");
    expect([root, /^#!\[(feature|register_tool)\b/m.test(source)]).toEqual([root, false]);
  }
  const { RUSTC_BOOTSTRAP: _, ...env } = process.env;
  const dir = fixture("stable-bindings");
  const p = Bun.spawnSync(["react/build.sh", "-o", join(dir, "libreact.rmeta")], { cwd: join(import.meta.dir, ".."), env: { ...env, RUST_JS_COMPILER: compiler } });
  expect(p.stderr.toString()).not.toContain("error");
  expect(p.exitCode).toBe(0);
  for (const name of ["libjs.rmeta", "libwebapi.rmeta", "libreact.rmeta"]) expect(Bun.file(join(dir, name)).size).toBeGreaterThan(0);
});

// A program is plain stable Rust too, for a user's own `cargo check` and
// editor (ADR 0113): a plain rustc compiles the crates it uses, which leave
// out rust-js's attributes, `cfg_attr(rust_js, ..)`, and the vite-react app,
// to which `jsx!` is react's placeholder, and `js::import!` nothing.
test("a plain stable rustc compiles the template's app and the crates it uses", () => {
  const { RUSTC_BOOTSTRAP: _, ...env } = process.env;
  const dir = fixture("plain-rustc");
  const repo = join(import.meta.dir, "..");
  const rustc = (source: string, name: string, externs: string[]) => {
    const p = Bun.spawnSync([
      "rustc", "--edition=2024", "--crate-type=lib", "--emit=metadata", "--target=wasm32-unknown-unknown", `--crate-name=${name}`,
      join(repo, source), "-o", join(dir, `lib${name}.rmeta`), "-L", dir,
      ...externs.flatMap((e) => ["--extern", `${e}=${join(dir, `lib${e}.rmeta`)}`]),
    ], { cwd: repo, env });
    return [name, p.exitCode, p.stderr.toString().split("\n").filter((line) => line.startsWith("error")).slice(0, 3)];
  };
  expect(rustc("builtins/src/lib.rs", "js", [])).toEqual(["js", 0, []]);
  expect(rustc("webapi/src/lib.rs", "webapi", ["js"])).toEqual(["webapi", 0, []]);
  expect(rustc("react/src/lib.rs", "react", ["js", "webapi"])).toEqual(["react", 0, []]);
  expect(rustc("examples/vite-react/src/App.rs", "app", ["js", "webapi", "react"])).toEqual(["app", 0, []]);
});

// An editor's rust-analyzer checks the app as Cargo does, with a plain
// stable rustc: the example's Cargo.toml has App.rs, and the crates it uses.
test("a plain `cargo check` checks the vite-react example", () => {
  const dir = join(fixture("cargo-check-example"), "target");
  const example = join(root, "examples", "vite-react");
  const p = runSync(["cargo", "check", "--offline", "--quiet", "--manifest-path", join(example, "Cargo.toml"), "--target-dir", dir], example, 300_000, { RUSTC_BOOTSTRAP: undefined });
  // No error, and no warning of the crates it uses: the app's are of what's
  // used only in JSX, which react's placeholder `jsx!` doesn't look inside.
  const warnings = p.stderr.split("\n").filter((line) => /^(warning|error)\b/.test(line) && !/\(lib\) generated \d+ warnings?/.test(line));
  expect(warnings.filter((line) => !/^warning: (unused variable: |static `\w+` is never used)/.test(line))).toEqual([]);
  expect(p.stderr).not.toMatch(/`rust-js-\w+` \(lib\) generated/);
  expect(p.code).toBe(0);
}, 300_000);
