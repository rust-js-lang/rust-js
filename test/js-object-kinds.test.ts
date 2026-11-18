// An object's kind and its test, `js::classify(value)`'s (ADR 0666): an
// object or an array one weak key, `kind.object()`; a property read where
// the test holds one conditional; and the object tested beside a closure's
// own of its name, which is the tested value, read where it's tested. Split
// from test/js-object.test.ts, which was at the file cap.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target];

/** `source` compiled, its JS, the module it is, loaded, and what rustc
 * warned of, a line each. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: any; warnings: string[] }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  const warnings = built.stderr.split("\n").filter((line) => line.startsWith("warning:") && !/warnings? emitted/.test(line));
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")), warnings };
}

// ADR 0910: what `typeof value === "object" && value !== null` takes, an
// object or an array, is one value, `kind.object()`, a `&JsObject` that
// keys a weak map, as React's ReactCapturedValue keys `CapturedStacks` by
// any object it's given.
test("an object or an array is one weak key, kind.object()", async () => {
  const { js, lib } = await compiled("object-keys", `use std::rc::Rc;

use js::{JsObject, Unknown, WeakMap};

thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "CapturedStacks")]
    static CAPTURED_STACKS: &'static WeakMap<JsObject, Rc<CapturedValue>> = js::weak_map::new();
}

pub struct CapturedValue {
    pub value: Option<&'static Unknown>,
    pub stack: String,
}

pub fn create_captured_value_at_fiber(value: Option<&'static Unknown>, stack: &str) -> Rc<CapturedValue> {
    // If the value is an error, call this function immediately after it is thrown
    // so the stack is accurate.
    if let Some(object) = js::classify(value).object() {
        let existing = CAPTURED_STACKS.with(|captured_stacks| captured_stacks.get(object));
        if let Some(existing) = existing {
            return existing;
        }
        let captured = Rc::new(CapturedValue {
            value,
            stack: stack.to_string(),
        });
        CAPTURED_STACKS.with(|captured_stacks| captured_stacks.set(object, Rc::clone(&captured)));
        captured
    } else {
        Rc::new(CapturedValue {
            value,
            stack: stack.to_string(),
        })
    }
}

pub fn objects(values: &[Option<&'static Unknown>]) -> Vec<bool> {
    values.iter().map(|&value| js::classify(value).object().is_some()).collect()
}
`);
  expect(js).toContain(`  // If the value is an error, call this function immediately after it is thrown
  // so the stack is accurate.
  if (typeof value === "object" && value !== null) {
    const existing = CapturedStacks.get(value);`);
  expect(js).toContain("CapturedStacks.set(value, captured);");
  // Its presence read in place is its test.
  expect(js).toContain('return values.map((value) => typeof value === "object" && value !== null);');
  const error = new Error("boom");
  const children = ["a"];
  const first = lib.createCapturedValueAtFiber(error, "at A");
  const again = lib.createCapturedValueAtFiber(error, "at B");
  const array = lib.createCapturedValueAtFiber(children, "at C");
  const arrayAgain = lib.createCapturedValueAtFiber(children, "at D");
  const text = lib.createCapturedValueAtFiber("text", "at E");
  const textAgain = lib.createCapturedValueAtFiber("text", "at F");
  expect([first === again, again.stack, array === arrayAgain, arrayAgain.stack, text === textAgain, textAgain.stack]).toEqual([true, "at A", true, "at C", false, "at F"]);
  expect(lib.objects([{}, [], null, undefined, "s", 1, () => 1, Symbol.iterator])).toEqual([true, true, false, false, false, false, false, false]);
});

// React's HotReloading reads `typeof nextType === 'object' && nextType !==
// null ? nextType.$$typeof : null`; its port, `if let Some(next_type) =
// js::classify(next_type).object() { js::get(next_type, "$$typeof") } else
// { None }`, the `Option` `kind.object()` made where it's tested: that
// test, one conditional, not `v = t ? v : undefined; const r = v?.$$typeof;`
// (ADR 0910, amended).
test("an object's property read where its test holds is one conditional", async () => {
  const { js, lib } = await compiled("object-read-where-tested", `use js::Unknown;

pub fn type_of_type(next_type: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let typeof_next_type = if let Some(next_type) = js::classify(next_type).object() {
        js::get(next_type, "\$\$typeof")
    } else {
        None
    };
    typeof_next_type
}
`);
  expect(js).toContain(
    'const typeofNextType =\n    typeof nextType === "object" && nextType !== null ? nextType.$$typeof : undefined;\n',
  );
  const tag = Symbol("tag");
  expect([lib.typeOfType({ $$typeof: tag }), lib.typeOfType(null), lib.typeOfType(3), lib.typeOfType(undefined)]).toEqual([tag, undefined, undefined, undefined]);
});

// ReactFlightReplyClient's resolveToJSON tests `if let Some(object) =
// js::classify(value).object()`, and a closure in it names its own
// `value`, its `Result` that reads nothing of `object`: the `if` is that
// test still, `object` read as `value` (rd514). A closure of its own
// `value` that reads `object` keeps the `const`, which it reads. The
// first closure's arms do other JS: of the same, `log(&text)` and
// `log(&reason)`, its `match` is that JS once, `log(checked(part)._0)`
// (b9ac9eca3), which holds no `value`, and this checked nothing (the
// Nightly of 3c71ee8f9).
test("an object tested beside a closure's own of its name is its test", async () => {
  const { js, lib } = await compiled("object-tested-beside-own", `use js::Unknown;

fn checked(value: Option<&'static Unknown>) -> Result<String, String> {
    match js::classify(value) {
        js::Kind::Null => Err("null".to_string()),
        _ => Ok("ok".to_string()),
    }
}

pub fn resolve(value: Option<&'static Unknown>, log: &dyn Fn(&str), then: &dyn Fn(&dyn Fn(Option<&'static Unknown>))) -> bool {
    if let Some(object) = js::classify(value).object() {
        log(&js::type_of(js::get(object, "x")));
        then(&move |part| match checked(part) {
            Ok(text) => log(&text),
            Err(reason) => log(&format!("{reason}!")),
        });
        return true;
    }
    false
}

pub fn resolve_read(value: Option<&'static Unknown>, log: &dyn Fn(&str), then: &dyn Fn(&dyn Fn(Option<&'static Unknown>))) -> bool {
    if let Some(object) = js::classify(value).object() {
        then(&move |part| match checked(part) {
            Ok(text) => log(&(text + &js::type_of(js::get(object, "x")))),
            Err(reason) => log(&reason),
        });
        return true;
    }
    false
}
`);
  expect(js).toContain('export function resolve(value, log, then) {\n  if (typeof value === "object" && value !== null) {\n    log(typeof value.x);\n');
  expect(js).toContain("const object =");
  const seen: string[] = [];
  const log = (text: string) => seen.push(text);
  const then = (f: (part: unknown) => void) => { f(1); f(null); };
  expect([lib.resolve({ x: 1 }, log, then), lib.resolve(2, log, then), lib.resolveRead({ x: "s" }, log, then)]).toEqual([true, false, true]);
  expect(seen).toEqual(["number", "ok", "null!", "okstring", "null"]);
});
