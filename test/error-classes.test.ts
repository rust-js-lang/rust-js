// A class that extends JS's Error (#1808, ADR 1158): a `#[rust_js::class]`
// struct that implements `js::ErrorClass` is an ES class, `class VGPUError
// extends Error`, as ADR 1041's `Component` makes one extend React's. Its
// `message` and `cause` fields are what Error's constructor sets, through
// `super`, so each is the non-enumerable own property JS's own subclass
// has; every other field is set after. Upstream is vercel-labs/vgpu's
// packages/core/src/errors.ts, run beside it to compare.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { es2022, type Setting, typeChecked } from "./type-checked";

beforeAll(buildReact, 600_000);

// What this file's checks of its declarations are made under (`typeChecked`).
const checking: Setting = { ...es2022, lib: ["es2022"], types: [] };

const flags = (dir: string) => ["--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const program = `use js::{ErrorClass, Unknown};

/// What a failure is made of.
pub struct VgpuErrorData {
    pub code: String,
    pub message: String,
    pub severity: Option<&'static str>,
    pub fix: Option<String>,
    pub cause: Option<&'static Unknown>,
}

/// A failure of the pipeline, with what to do about it.
#[cfg_attr(rust_js, rust_js::class, rust_js::name = "VGPUError")]
pub struct VgpuError {
    message: String,
    cause: Option<&'static Unknown>,
    name: &'static str,
    pub code: String,
    pub severity: &'static str,
    pub fix: Option<String>,
}

impl ErrorClass for VgpuError {}

#[cfg_attr(rust_js, rust_js::class)]
impl VgpuError {
    pub fn new(data: VgpuErrorData) -> Self {
        Self {
            message: data.message,
            cause: data.cause,
            name: "VGPUError",
            code: data.code,
            severity: data.severity.unwrap_or("error"),
            fix: data.fix,
        }
    }
}

/// One of no cause, as new Error(message) has none.
#[cfg_attr(rust_js, rust_js::class)]
pub struct Plain {
    pub message: String,
}

impl ErrorClass for Plain {}

#[cfg_attr(rust_js, rust_js::class)]
impl Plain {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

/// One whose cause is none, there as Rust's field is.
#[cfg_attr(rust_js, rust_js::class)]
pub struct Uncaused {
    pub message: String,
    pub cause: Option<&'static Unknown>,
}

impl ErrorClass for Uncaused {}

#[cfg_attr(rust_js, rust_js::class)]
impl Uncaused {
    pub fn new(prefix: &str, count: u32) -> Self {
        Self {
            message: format!("{prefix} {count}"),
            cause: None,
        }
    }
}

thread_local! {
    static MADE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn next() -> u32 {
    MADE.with(|made| {
        made.set(made.get() + 1);
        made.get()
    })
}

fn label(prefix: &str) -> String {
    format!("{prefix}{}", next())
}

/// One whose field before its message runs code first, as Rust's order is.
#[cfg_attr(rust_js, rust_js::class)]
pub struct Counted {
    pub code: u32,
    pub message: String,
}

impl ErrorClass for Counted {}

#[cfg_attr(rust_js, rust_js::class)]
impl Counted {
    pub fn new(prefix: &str) -> Self {
        Self {
            code: next(),
            message: label(prefix),
        }
    }
}

pub fn unsupported(missing: &[&str]) -> VgpuError {
    let names: Vec<String> = missing.iter().map(|name| format!("\\"{name}\\"")).collect();
    VgpuError::new(VgpuErrorData {
        code: "VGPU-FEATURE-UNSUPPORTED".to_string(),
        message: format!("Adapter does not support requested feature(s): {}.", names.join(", ")),
        severity: None,
        fix: Some("Remove the unsupported name(s).".to_string()),
        cause: None,
    })
}

/// Throws one, as upstream's throw of unsupportedFeaturesError(missing).
pub fn validate(missing: &[&str]) {
    if !missing.is_empty() {
        js::throw(Some(js::unknown(unsupported(missing).error())));
    }
}

/// What JS sees of it: the stack it keeps, and the error it is.
pub fn described(error: &'static VgpuError) -> (bool, String) {
    (error.stack().is_some(), js::js_error::to_string(error.error()))
}
`;

// Upstream's, as written.
const upstream = `export class VGPUError extends Error {
  constructor(data) {
    super(data.message, { cause: data.cause });
    this.name = "VGPUError";
    this.code = data.code;
    this.severity = data.severity ?? "error";
    this.fix = data.fix;
  }
}
`;

test("a class that implements js::ErrorClass is JS's own subclass of Error", async () => {
  const dir = fixture("error-classes");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, ...flags(dir)]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(upstream);
  expect(js).toContain("export class Plain extends Error {\n  constructor(message) {\n    super(message);\n  }\n}");
  writeFileSync(join(dir, "upstream.js"), upstream);
  const ours = await import(join(dir, "lib.js"));
  const theirs = await import(join(dir, "upstream.js"));
  const cause = new TypeError("native");
  const data = { code: "C", message: "m", severity: undefined, fix: undefined, cause };
  const [a, b] = [new ours.VGPUError(data), new theirs.VGPUError(data)];
  // What JS tells of each, the same.
  const seen = (e: any) => ({
    keys: Object.keys(e),
    json: JSON.stringify(e),
    own: Object.getOwnPropertyNames(e).sort(),
    message: Object.getOwnPropertyDescriptor(e, "message"),
    cause: Object.getOwnPropertyDescriptor(e, "cause"),
    isError: e instanceof Error,
    text: String(e),
    stackNamed: typeof e.stack === "string" && e.stack.startsWith("VGPUError: m"),
  });
  expect(seen(a)).toEqual(seen(b));
  expect(a).toBeInstanceOf(ours.VGPUError);
  expect(a.cause).toBe(cause);
  expect(Object.getPrototypeOf(ours.VGPUError.prototype)).toBe(Error.prototype);
  // No cause given, none there, as `new Error(message)` has none.
  expect(Object.hasOwn(new ours.Plain("p"), "cause")).toBe(false);
  expect(Object.keys(new ours.Plain("p"))).toEqual([]);
  // A cause field of none: there, `undefined`, as Rust's field is.
  expect(js).toContain("  constructor(prefix, count) {\n    super(`${prefix} ${count}`, { cause: undefined });\n");
  expect(new ours.Uncaused("u", 2).message).toBe("u 2");
  expect(Object.getOwnPropertyDescriptor(new ours.Uncaused("u", 2), "cause")).toEqual({ value: undefined, writable: true, enumerable: false, configurable: true });
  // Thrown, caught as the Error it is.
  let thrown: any;
  try {
    ours.validate(["timestamp-query"]);
  } catch (e) {
    thrown = e;
  }
  expect(thrown).toBeInstanceOf(ours.VGPUError);
  expect([thrown.code, thrown.severity, thrown.message]).toEqual(["VGPU-FEATURE-UNSUPPORTED", "error", 'Adapter does not support requested feature(s): "timestamp-query".']);
  expect(ours.described(thrown)).toEqual([true, 'VGPUError: Adapter does not support requested feature(s): "timestamp-query".']);
  // A field before its message that runs code, held first: Rust's order.
  expect(js).toContain("    const code = next();\n    super(label(prefix));\n    this.code = code;\n");
  const counted = new ours.Counted("x");
  expect([counted.code, counted.message, Object.keys(counted)]).toEqual([1, "x2", ["code"]]);
  // Declared as the class it is, an Error to TypeScript too.
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export declare class VGPUError extends Error {\n    constructor(data: VgpuErrorData);\n    message: string;\n    cause?: unknown;\n");
  writeFileSync(join(dir, "consumer.ts"), `import { VGPUError } from "./lib.js";
const error = new VGPUError({ code: "C", message: "m", cause: new Error("native") });
export const seen: Error = error;
export const code: string = error.code;
export const cause: unknown = error.cause;
export const stack: string | undefined = error.stack;
// @ts-expect-error A code is text.
export const wrong: number = error.code;
`);
  expect(typeChecked(dir, "consumer.ts", checking)).toEqual([]);
});

test("what Error's constructor sets is refused of another shape, saying what to write", () => {
  const dir = fixture("error-classes-refused");
  const cases: [string, string][] = [
    [`#[cfg_attr(rust_js, rust_js::class)]
pub struct Coded {
    message: u32,
}

impl js::ErrorClass for Coded {}

#[cfg_attr(rust_js, rust_js::class)]
impl Coded {
    pub fn new(message: u32) -> Self {
        Self { message }
    }
}`, "that extends `Error` has a `message` of text"],
    [`#[cfg_attr(rust_js, rust_js::class)]
pub struct Unmade {
    message: Option<String>,
}

impl js::ErrorClass for Unmade {}`, "that extends `Error` writes its constructor"],
  ];
  for (const [body, refusal] of cases) {
    writeFileSync(join(dir, "lib.rs"), `${body}\n`);
    const ran = runSync([compiler, ...flags(dir)], root, 60_000);
    expect([body, ran.code === 0, ran.stderr.includes(refusal)]).toEqual([body, false, true]);
  }
});
