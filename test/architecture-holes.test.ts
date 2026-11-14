// The holes' rules of the architecture (#1671 change 4, M1): an expression
// that isn't JS yet is a `Hole`, which no tree is printed with, and JS's
// own `undefined` is a value, said so where it's made. Split from
// architecture.test.ts at its cap (a test file is at most 1500 lines).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// The compiler's sources, in name order, as macOS lists them and Linux,
// CI's, doesn't.
const sources = readdirSync(join(root, "src"), { recursive: true, encoding: "utf8" })
  .filter((file) => file.endsWith(".rs"))
  .map((file) => `src/${file}`)
  .sort();

// Each `undefined` says which it is: JS's value, `Expr::js_undefined()`, or
// what a value was taken out of, `Expr::taken()`, a hole `js::validate`
// refuses where it's left. A bare `Expr::undefined()` said neither, and a
// take that was never put back printed `undefined` where a value was.
test("each undefined says whether it's a value or a hole", () => {
  const bare = sources.filter((file) => /Expr::undefined\b/.test(read(file)));
  expect(bare).toEqual([]);
  // What `mem::replace` takes a value out of is a hole, never JS's value,
  // but a list's item taken where the list is still made whole, whose
  // place JS's `undefined` holds and nothing reads (`take_field`).
  const printed = new Set(["src/lower/mir/operands.rs: replace(&mut items[field], Value::Expr(Expr::js_undefined()"]);
  const taken = sources.flatMap((file) =>
    [...read(file).matchAll(/replace\(\s*[^;]*?,\s*(?:Value::Expr\()?Expr::js_undefined\(\)/g)]
      .map((m) => `${file}: ${m[0].replaceAll(/\s+/g, " ")}`)
      .filter((found) => !printed.has(found))
      .map((found) => `${found}: take it out with \`Expr::taken()\``),
  );
  expect(taken).toEqual([]);
});

// A library's item asked for is a hole, `Hole::Asked`, as the crate is
// lowered again with it: never JS's `undefined`, which a crate that
// failed to ask again would have printed where its item is called.
test("a library's item asked for is a hole", () => {
  // Each ask, and what's given back where it's asked: a hole, as the crate
  // is lowered again with it, never JS's `undefined`.
  const asks = sources.flatMap((file) =>
    [...read(file).matchAll(/self\.asked_of_libraries\.borrow_mut\(\)\.push\([^;]*;\n\s*return ([^;]*);/g)].map((m) => [file, m[1]]),
  );
  expect(asks.length).toBe(3);
  expect(asks.filter(([, given]) => !given.includes("Expr::hole(js::Hole::Asked)"))).toEqual([]);
});
