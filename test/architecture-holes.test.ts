// The holes' rules of the architecture (#1671 change 4, M1): an expression
// that isn't JS yet is a `Hole`, which no tree is printed with, and JS's
// own `undefined` is a value, said so where it's made. Split from
// architecture.test.ts at its cap (a test file is at most 1500 lines).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);
const sources = files("src").filter((file) => file.endsWith(".rs"));

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
  const items = read("src/lower/items.rs");
  expect(items).toContain("self.krate.asked.borrow_mut().push((def_id, self.module));\n            return Expr::hole(js::Hole::Asked);");
});
