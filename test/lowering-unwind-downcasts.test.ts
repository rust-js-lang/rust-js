// What an unwind's payload downcasts to: the `JsError` thrown, every
// unwind's, and what's given the thrown downcast, given it as it is. Split
// from lowering-unwinds.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS, and where it is. */
function compiled(name: string, rust: string, flags: string[] = []): { js: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target, ...flags]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), dir };
}

// React's getOwnerStackByComponentInfoInDev reads what it caught, `catch
// (x) { return '..' + x.message + '\n' + x.stack; }`: every unwind's
// payload downcasts as the `&JsError` thrown (ADR 0710), a typed one's its
// `$Unwinding`, a falsy throw's itself, so its `None` arm is dead and
// dropped, and the `catch` is of what was thrown. A typed payload still
// downcasts to its type, and is resumed as the same `$Unwinding`.
test("every unwind's payload downcasts as the JsError thrown", async () => {
  const { js, dir } = compiled("unwind-every-thrown", `use js::JsError;
use std::any::Any;
use std::panic::{catch_unwind, resume_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.render")]
fn render() -> String {
    unreachable!()
}

pub fn info() -> String {
    match catch_unwind(render) {
        Ok(info) => info,
        Err(x) => match x.downcast_ref::<&'static JsError>() {
            Some(x) => format!("\\nError generating stack: {}\\n{}", x.message(), x.stack().unwrap_or_default()),
            None => format!("\\nError generating stack: {}\\n", x.downcast_ref::<String>().cloned().unwrap_or_default()),
        },
    }
}

pub fn kind() -> String {
    match catch_unwind(render) {
        Ok(_) => String::from("none"),
        Err(x) => match x.downcast_ref::<&'static JsError>() {
            Some(e) => format!("{e:?}"),
            None => String::from("not thrown"),
        },
    }
}

pub fn control() -> Option<&'static JsError> {
    let mut control = None;
    if let Err(x) = catch_unwind(render) {
        control = x.downcast_ref::<&'static JsError>().copied();
    }
    control
}

pub fn typed(n: i32) -> String {
    match catch_unwind(|| -> String { resume_unwind(Box::new(n)) }) {
        Ok(s) => s,
        Err(x) => match x.downcast_ref::<&'static JsError>() {
            Some(e) => format!("{} {}", e.message(), x.downcast_ref::<i32>().copied().unwrap_or_default()),
            None => String::from("none"),
        },
    }
}

pub fn is_thrown(n: i32) -> bool {
    catch_unwind(|| -> u32 { resume_unwind(Box::new(n)) })
        .err()
        .is_some_and(|payload| payload.is::<&JsError>() && payload.is::<i32>())
}

pub fn boxed(n: i32) -> String {
    match catch_unwind(|| -> u32 { resume_unwind(Box::new(n)) }) {
        Ok(_) => String::new(),
        Err(payload) => match payload.downcast::<&JsError>() {
            Ok(error) => error.message(),
            Err(_) => String::from("not an Error"),
        },
    }
}

pub fn own() -> Option<i32> {
    let payload: Box<dyn Any + Send> = Box::new(7i32);
    payload.downcast_ref::<i32>().copied()
}

pub fn typed_throw(n: i32) -> String {
    resume_unwind(Box::new(n))
}

fn resume(payload: Box<dyn Any + Send>) -> String {
    resume_unwind(payload)
}

pub fn passed() -> String {
    match catch_unwind(render) {
        Ok(s) => s,
        Err(payload) => resume(payload),
    }
}

fn message_of(payload: &(dyn Any + Send)) -> String {
    payload.downcast_ref::<&'static JsError>().map(|e| e.message()).unwrap_or_default()
}

pub fn passed_message() -> String {
    match catch_unwind(render) {
        Ok(s) => s,
        Err(payload) => message_of(&*payload),
    }
}

fn text_of(payload: &(dyn Any + Send)) -> String {
    match payload.downcast_ref::<&str>() {
        Some(text) => format!("a panic, {text}"),
        None => String::from("not a panic"),
    }
}

pub fn passed_text() -> String {
    match catch_unwind(render) {
        Ok(s) => s,
        Err(payload) => text_of(&*payload),
    }
}

pub fn tried() -> Option<String> {
    let Err(x) = catch_unwind(render) else {
        return None;
    };
    let e = x.downcast_ref::<&'static JsError>()?;
    Some(format!("{e:?}"))
}

pub fn reset(n: u32) -> bool {
    let Err(x) = catch_unwind(render) else {
        return false;
    };
    let mut e = x.downcast_ref::<&'static JsError>();
    if n > 1 {
        e = None;
    }
    match e {
        Some(_) => true,
        None => false,
    }
}

fn widened(payload: Box<dyn Any + Send>) -> Box<dyn Any> {
    payload
}

pub fn of_any(own: bool) -> String {
    let any: Box<dyn Any> = if own {
        let error: &'static JsError = js::error::new(Some("own"), None);
        Box::new(error)
    } else {
        widened(catch_unwind(|| -> u32 { resume_unwind(Box::new(4i32)) }).unwrap_err())
    };
    let found = any.downcast_ref::<&'static JsError>().map(|e| e.message());
    format!("{found:?} {}", any.is::<&JsError>())
}

pub fn of_any_boxed(own: bool) -> String {
    let any: Box<dyn Any> = if own {
        let error: &'static JsError = js::error::new(Some("own"), None);
        Box::new(error)
    } else {
        widened(catch_unwind(|| -> u32 { resume_unwind(Box::new(4i32)) }).unwrap_err())
    };
    match any.downcast::<&JsError>() {
        Ok(e) => e.message(),
        Err(_) => String::from("not an Error"),
    }
}
`);
  // The `catch` is of what was thrown: no `$unwound`, no test, no `None`.
  expect(js).toContain(`  } catch (x) {
    $catchable(x);
    return \`\\nError generating stack: \${x.message}\\n\${x.stack ?? ""}\`;
  }`);
  expect(js).toContain(`  } catch (x) {
    $catchable(x);
    control = x;
  }`);
  expect(js).not.toContain("not thrown");
  // The payload is made where it's first read, after what reads only
  // what was thrown (rd550).
  expect(js).toContain("    const e = thrown;\n    const x = $unwound(thrown);\n    return `${e.message} ");
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  const thrown = new TypeError("no props");
  g.render = () => {
    throw thrown;
  };
  expect(lib.info()).toBe(`\nError generating stack: no props\n${thrown.stack}`);
  expect([lib.control(), lib.passedMessage()]).toEqual([thrown, "no props"]);
  g.render = () => {
    throw new Error("a panic's");
  };
  expect(lib.info()).toStartWith("\nError generating stack: a panic's\nError: a panic's");
  // Passed on, a payload is what was thrown, a panic's message too, and
  // resumed, it's thrown again as itself.
  for (const error of [new TypeError("no props"), new Error("a panic's")]) {
    g.render = () => {
      throw error;
    };
    expect([lib.passedMessage(), lib.passedText()]).toEqual([
      error.message,
      error instanceof TypeError ? "not a panic" : "a panic, a panic's",
    ]);
    let resumed: unknown;
    try {
      lib.passed();
    } catch (e) {
      resumed = e;
    }
    expect(resumed).toBe(error);
  }
  // A falsy throw is what was thrown too: the `Some` arm's.
  const kinds = [];
  for (const value of [undefined, null, 0, "", false, 5]) {
    g.render = () => {
      throw value;
    };
    kinds.push(lib.kind());
  }
  expect(kinds).toEqual(["undefined", "null", "0", "", "false", "5"]);
  // `?` of it goes on; a variable set again is what it's set to.
  expect([lib.tried(), lib.reset(1), lib.reset(2)]).toEqual(["5", true, false]);
  g.render = () => {
    throw undefined;
  };
  expect(lib.tried()).toBe("undefined");
  // A `dyn Any` may hold a `&JsError` of its own: an unwind's is what was
  // thrown, and one of its own itself.
  expect([lib.ofAny(true), lib.ofAny(false)]).toEqual(['Some("own") true', 'Some("Box<dyn Any>") true']);
  expect([lib.ofAnyBoxed(true), lib.ofAnyBoxed(false)]).toEqual(["own", "Box<dyn Any>"]);
  // A typed payload is its `$Unwinding`, and its type's still.
  expect(lib.typed(5)).toBe("Box<dyn Any> 5");
  expect([lib.isThrown(5), lib.boxed(5), lib.own()]).toEqual([true, "Box<dyn Any>", 7]);
  // Resumed, it's the same `$Unwinding` again.
  let first: unknown;
  g.render = () => {
    try {
      lib.typedThrow(3);
    } catch (e) {
      first = e;
    }
    throw first;
  };
  let caught: unknown;
  try {
    lib.passed();
  } catch (e) {
    caught = e;
  }
  expect(caught).toBeInstanceOf(Error);
  expect(caught).toBe(first);
  delete g.render;
});

// React's describeBuiltInComponentFrame reads the stack of what it threw,
// `catch (x) { const match = x.stack.trim().match(..); }`. The downcast
// of what's caught is always `Some` (ADR 0710), so what's given its value,
// `and_then(|x| x.stack())`'s closure, `map`'s and `is_some_and`'s, is
// given it, as Rust gives a `Some`'s: `x.stack`, never `x?.stack` (ADR
// 0710, amended).
test("what's given the thrown downcast is given it as it is", async () => {
  const { js, dir } = compiled("unwind-given-thrown", `use js::JsError;
use std::panic::catch_unwind;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.render")]
fn render() -> u32 {
    unreachable!()
}

pub fn prefix() -> String {
    let Err(x) = catch_unwind(|| panic!(""));
    let stack = x
        .downcast_ref::<&'static JsError>()
        .and_then(|x| x.stack())
        .unwrap_or_default();
    stack.trim().to_string()
}

pub fn control() -> Option<&'static JsError> {
    let Err(x) = catch_unwind(|| panic!(""));
    x.downcast_ref::<&'static JsError>().copied()
}

pub fn message() -> Option<String> {
    match catch_unwind(render) {
        Ok(_) => None,
        Err(payload) => payload.downcast_ref::<&'static JsError>().map(|e| e.message()),
    }
}

pub fn nested() -> Option<Option<u32>> {
    match catch_unwind(render) {
        Ok(_) => None,
        Err(payload) => payload.downcast_ref::<&'static JsError>().map(|_| None),
    }
}

pub fn named(name: &str) -> bool {
    match catch_unwind(render) {
        Ok(_) => false,
        Err(payload) => payload
            .downcast_ref::<&'static JsError>()
            .is_some_and(|e| e.message() == name),
    }
}
`);
  expect(js).toContain(`  } catch (x) {
    $catchable(x);
    const stack = x.stack ?? "";
    return $trim(stack);
  }`);
  expect(js).toContain("  } catch (x) {\n    $catchable(x);\n    return x ?? undefined;\n  }");
  expect(js).toContain("  } catch (payload) {\n    $catchable(payload);\n    return payload.message;\n  }");
  expect(js).toContain("  } catch (payload) {\n    $catchable(payload);\n    return payload.message === name;\n  }");
  expect(js).not.toContain("?.");
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as Record<string, unknown>;
  expect(lib.prefix()).toStartWith("Error");
  expect(lib.control()).toBeInstanceOf(Error);
  g.render = () => {
    throw new TypeError("no props");
  };
  expect([lib.message(), lib.named("no props"), lib.named("other")]).toEqual(["no props", true, false]);
  // `map`'s `Some` of a `None` is boxed, as an `Option` of what looks
  // like `None` is (ADR 0051).
  expect(lib.nested()).not.toBe(undefined);
  g.render = () => {
    throw 5;
  };
  expect([lib.message(), lib.named("5")]).toEqual([undefined, false]);
  delete g.render;
});
