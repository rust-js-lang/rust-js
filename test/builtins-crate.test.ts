// The builtins crate's functions, each JS's own (ADR 0102): `JSON`,
// strings, objects and errors, timers, `toFixed` and `Math`, compiled from
// test/builtins.rs and run. Split from test/compiler.test.ts by concept
// (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { buildCompiler, buildReact, compiler, fixture, run, target } from "./support";
import { withoutBoundary } from "./without-boundary";

let builtins: Record<string, (...args: any[]) => any>;
let builtinsJs = "";

beforeAll(async () => {
  buildCompiler();
  // The webapi crate is used from its metadata (ADR 0024).
  buildReact();
  builtinsJs = join(fixture("builtins-crate"), "builtins.js");
  run([compiler, "test/builtins.rs", "-o", builtinsJs, "--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  builtins = await import(builtinsJs);
}, 600_000);

// JS's own, from the builtins crate (ADR 0102): `JSON.stringify` of a
// string, its JSON text and a JS string literal, and `replace` with a
// string, in which `$1` is a group.
test("the builtins crate's json::stringify and reg_exp::replace are JS's", () => {
  for (const text of ["plain", 'a "quote" and a \\ backslash', "a line\nbreak, a\ttab, a \u0001", "é and 😀", ""]) {
    expect(builtins.quoted(text)).toBe(JSON.stringify(text));
    expect(JSON.parse(builtins.quoted(text))).toBe(text);
  }
  expect(builtins.quoted('say "hi"\n')).toBe('"say \\"hi\\"\\n"');
  expect(builtins.replaced("a-b-c", "-", "g", "+")).toBe("a+b+c");
  expect(builtins.replaced("a-b-c", "-", "", "+")).toBe("a+b-c");
  expect(builtins.replaced("2026-09-30", "(\\d+)-(\\d+)-(\\d+)", "", "$3/$2/$1")).toBe("30/09/2026");
});

// JS's string methods (ADR 0247), as JS has them: by UTF-16 indexes, so an
// emoji is two, and a negative index of `slice` counts from the end.
test("the builtins crate's string functions are JS's string methods", () => {
  const text = " 😀ab ab ";
  const [parts, at, length] = builtins.parts(text, "ab");
  expect(parts).toEqual([text.slice(1, -1), text.slice(-2), text.substring(1, 3), text.substring(1), text.trim(), text.trimStart(), text.trimEnd()]);
  // A position from the end is a number, as one `index_of` found (rd343).
  expect(at).toEqual([text.indexOf("ab"), text.indexOf("ab", 2), text.lastIndexOf("ab"), text.lastIndexOf("ab", 5), text.charCodeAt(text.indexOf("ab") - 1)]);
  expect(length).toBe(text.length);
  // An index is a JS number, as TypeScript has it: one past it is `+ 1`.
  expect(builtins.fileName("a/b/c.txt")).toBe("c.txt");
  expect(readFileSync(builtinsJs, "utf8")).toContain('return path.slice(path.lastIndexOf("/") + 1);');
  expect(builtins.units("a😀")).toEqual(["a", "\ud83d", "\ude00"]);
  expect(builtins.replacedFirst("a-b-c", "-", "+")).toBe("a+b-c");
  expect(builtins.splitParts("a,b;c,")).toEqual([["a", "b;c", ""], ["a", "b", "c", ""]]);
  expect(builtins.revived('{"a":1,"secret":2,"b":{"secret":3}}')).toEqual({ a: 1, b: {} });
  const js = withoutBoundary(readFileSync(builtinsJs, "utf8"));
  expect(js).toContain('return [text.split(","), text.split(/[,;]/)];');
  expect(js).toContain("return JSON.parse(text, hidden);");
  expect(js).toContain("  if (key in value) {\n    return value[key];\n  }");
  expect([builtins.named({ a: 1 }, "a"), builtins.named({ a: 1 }, "b")]).toEqual([1, undefined]);
  expect(() => builtins.revived("{")).toThrow();
  expect([builtins.holdsNone({ undefined: 1 }, undefined), builtins.holdsNone({}, undefined), builtins.holdsNone({ a: 1 }, "a")]).toEqual([true, false, true]);
  expect(js).toContain("export function holdsNone(value, key) {\n  return key in value;\n}");
  expect(js).toContain("return { children };");
  expect(builtins.wrapped("text")).toEqual({ children: "text" });
  expect(js).toContain('  const files = {};\n  files["/App.js"] = 1;');
  expect(builtins.filled()).toEqual({ "/App.js": 1 });
  expect(js).toContain("export function asUnknown(text) {\n  return text;\n}");
  expect(js).toContain("return key in value;");
  expect([builtins.holds({ a: 1 }, "a"), builtins.holds({}, "toString"), builtins.holds({}, "b")]).toEqual([true, true, false]);
  for (const call of ["text.slice(1, -1)", "text.slice(-2)", "text.substring(1, 3)", "text.substring(1)", "text.trim()", "text.trimStart()", "text.trimEnd()", 'text.indexOf(part)', "text.indexOf(part, 2)", "text.lastIndexOf(part)", "text.lastIndexOf(part, 5)", "text.charCodeAt(text.indexOf(part) - 1)", "text.length", "text.charAt(i)", "text.replace(from, to)"]) {
    expect(js).toContain(call);
  }
});

// `Object.fromEntries` and `Object.is`, and an `Error`'s message, for what
// was thrown (ADR 0102).
test("the builtins crate's object and js_error functions are JS's", async () => {
  expect(builtins.attributes("Rust source", 2)).toEqual({ "aria-label": "Rust source", "aria-level": "2" });
  expect(builtins.identical(true)).toEqual([true, true]);
  expect(builtins.identical(false)).toEqual([false, true]);
  // Its message is the engine's: "URI malformed" in V8, "URI error" in Bun.
  const message = (() => {
    try {
      decodeURIComponent("%E0%A4%A");
    } catch (e) {
      return (e as Error).message;
    }
  })();
  expect(await builtins.thrown()).toEqual([message, "not an Error"]);
  // A future is the promise it is, unawaited where it's handed on.
  expect(await builtins.promised(4)).toBe(8);
  const js = await Bun.file(builtinsJs).text();
  expect(js).toContain("const pending = doubled(n);");
});

// Timers, globals of every JS runtime, as ReScript's standard library has
// them (ADR 0102): a closure after a time, or every so often, until cleared.
test("the builtins crate's timers run a closure, and clearing one stops it", async () => {
  await new Promise<void>((resolve) => builtins.after(5, resolve));
  let fired = false;
  builtins.cancelled(5, () => {
    fired = true;
  });
  let ticks = 0;
  // Its third tick awaited, not a time a loaded machine may give two in
  // (a push's check, 2026-11-27: 2 of 3 in 80 ms), then as long again
  // for a fourth that stopping it means never comes.
  const began = performance.now();
  await new Promise<void>((resolve) => {
    const id = builtins.every(5, () => {
      ticks += 1;
      if (ticks === 3) {
        builtins.stop(id);
        resolve();
      }
    });
  });
  await new Promise((resolve) => setTimeout(resolve, Math.max(20, performance.now() - began)));
  expect(fired).toBe(false);
  expect(ticks).toBe(3);
  await new Promise<void>((resolve) => builtins.at(Date.now() + 5.5, resolve));
  await new Promise<void>((resolve) => builtins.soon(resolve));
  const js = await Bun.file(builtinsJs).text();
  expect(js).toContain("setTimeout(done, at - Date.now());");
  // Its delay is optional, as JS's: left out where it's `None`.
  expect(js).toContain("  setTimeout(done);\n");
});

// `toFixed` is JS's rounding: a tie away from zero, where `format!` rounds it
// to even, as Rust does; `-0` without its sign, and `1e21` and up as
// `String(x)`. Between ties they agree (ADR 0102).
test("the builtins crate's number::to_fixed is JS's toFixed, not format!'s", () => {
  expect(builtins.fixed(2.5, 0)).toEqual(["3", "2"]);
  expect(builtins.fixed(1.25, 1)).toEqual(["1.3", "1.2"]);
  expect(builtins.fixed(-2.5, 0)).toEqual(["-3", "-2"]);
  expect(builtins.fixed(-0, 0)).toEqual(["0", "-0"]);
  expect(builtins.fixed(1e21, 2)[0]).toBe("1e+21");
  expect(builtins.fixed(1234.5678, 2)).toEqual(["1234.57", "1234.57"]);
});

// JS's Math, where its answer isn't Rust's method's, for a program that
// wants JS's: `round` a half up, `max` and `min` `NaN` of a `NaN` and
// `-0` below `0`, `sign` `0` of a zero; and `random`, which Rust has none of.
test("the builtins crate's math is JS's Math where Rust's methods differ", async () => {
  expect(builtins.rounded(-2.5)).toEqual([-2, -3]);
  expect(builtins.rounded(2.5)).toEqual([3, 3]);
  expect(builtins.rounded(-0.4)).toEqual([-0, -0]);
  const [max, rustMax, min, rustMin] = builtins.bounds(NaN, 1);
  expect([Number.isNaN(max), rustMax, Number.isNaN(min), rustMin]).toEqual([true, 1, true, 1]);
  expect(Object.is(builtins.bounds(-0, 0)[2], -0)).toBe(true);
  expect(builtins.signs(0)).toEqual([0, 1]);
  expect(Object.is(builtins.signs(-0)[0], -0)).toBe(true);
  const r = builtins.random();
  expect(r >= 0 && r < 1).toBe(true);
  const js = await Bun.file(builtinsJs).text();
  expect(js).toContain("Math.round(x)");
  expect(js).toContain("Math.max(a, b)");
  expect(js).toContain("Math.min(a, b)");
  expect(js).toContain("Math.sign(x)");
  expect(js).toContain("Math.random()");
});
