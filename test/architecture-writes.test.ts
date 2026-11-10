// Whether a local, a variable or a JS tree's name is written, split from
// architecture.test.ts at its cap (a test file is at most 1500 lines): each
// is one owner's answer, which every check asks.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// Whether a local is written again after its first value is
// `written_again`'s, which counts a parameter's argument as its first: a
// variable read as never changing (`alike`) and a `move` closure's capture
// each counted MIR's writes alone, so a `mut` parameter assigned once read
// as never changing, and what was moved out of it read its new value (#99).
test("a local written again is written_again's alone", () => {
  const counting = files("src/lower").filter((file) => file.endsWith(".rs"))
    .flatMap((file) => read(file).split("\n").filter((line) => /writes\[[^\]]+\]\s*(>|>=)\s*[12]\b/.test(line)).map((line) => `${file}: ${line.trim()}`));
  expect(counting).toEqual([]);
});

// A box is the value it holds (ADR 0023), so `*b = v` writes `b` as `b = v`
// does. Three checks of whether a local is written asked of the local
// alone and read a box written through as unchanged (#980, #997, #1000);
// four more built the same map of what each local is assigned once. What a
// place is the value of is body_queries/values.rs's (`value_local`,
// `assigned_once`, the step `through_box`), and whether a variable is set
// is `changes`'s, of THIR: a count or a map of whole assignments, or a THIR
// scan for `v = ..` of a `VarRef`, is written there alone. A site that
// counts by another rule is listed, with why.
test("whether a local is written is body_queries' alone, a box's content its own", () => {
  const listed = [
    // `as` casts' known values: each `=` of the variable counted, beside
    // `changes`, whose count of every change it must equal, through a box too.
    "src/lower/body_queries/whole_casts.rs: if let ExprKind::Assign { lhs, rhs } = expr.kind",
  ];
  const shapes = [
    /StatementKind::Assign\(assign\)[^{;]{0,120}?assign\.0\.projection\.is_empty\(\)\s*\{[^}]{0,200}?(\+= 1|contains_key\(&assign\.0\.local\))/g,
    /ExprKind::(?:Assign|AssignOp) \{ lhs[^}]*\}[^;]{0,240}?ExprKind::VarRef \{ id \}/g,
  ];
  const found = files("src/lower")
    .filter((file) => file.endsWith(".rs") && file !== "src/lower/body_queries/values.rs")
    .flatMap((file) => {
      const text = read(file);
      return shapes.flatMap((shape) =>
        [...text.matchAll(shape)].map((m) => `${file}: ${text.split("\n")[text.slice(0, m.index).split("\n").length - 1].trim()}`),
      );
    });
  expect(found.filter((site) => !listed.includes(site))).toEqual([]);
  expect(listed.filter((site) => !found.includes(site))).toEqual([]);
});

// Whether a tree sets a variable, and how often, is `js::Writes::sets` and
// `Writes::count` (ADR 1088, #523): no pass searches a tree's statements
// for an assignment of a variable by name of its own, which missed what
// another counted, `x += ..`, `(x = ..)`, a closure's or a handle. What
// each pass below looks for is a shape, the statement it rewrites, not
// whether the tree sets the variable.
test("whether a tree sets a variable is js::Writes' alone", () => {
  const shapes = [
    // The consecutive `name = value;`s hoisted before a `try` with it.
    "src/prepare/catches.rs: .take_while(|s| {",
    // The first statement of a `catch` that sets the variable afresh.
    "src/js/applied.rs: let first_set = caught.iter().position(|s| {",
    // Each mention of a temporary in a `switch` a case's assignment of it.
    "src/lower/mir/tidy/switches.rs: .filter(",
    // The first mention of a variable in a `try`, its assignment.
    "src/lower/mir/tidy/bindings.rs: StmtKind::Try(body, _) | StmtKind::TryCatch(body, _, _) => body.iter().find(|s| mentions(s) > 0).is_some_and(",
    // `let x; .. x = e;`'s first mention of `x`, the assignment it's declared at.
    "src/lower/mir/tidy/bindings.rs: .find(|&j| js::mentions_in(&stmts[j..=j], name) > 0)",
  ];
  const searching = /(\.any\(|\.filter\(|\.position\(|\.find\(|\.take_while\(|\|=)[^;]{0,160}?StmtKind::Assign\([^;]{0,200}?ExprKind::Var\(\w+\)\s*if\s+\*?\w+\s*==/g;
  const found = files("src")
    .filter((file) => file.endsWith(".rs") && file !== "src/js/writes.rs")
    .flatMap((file) => {
      const text = read(file);
      return [...text.matchAll(searching)].map((m) => `${file}: ${text.split("\n")[text.slice(0, m.index).split("\n").length - 1].trim()}`);
    });
  expect(found.filter((site) => !shapes.includes(site))).toEqual([]);
  expect(shapes.filter((site) => !found.includes(site))).toEqual([]);
});

// A `String`'s `extend` that steps its iterator is one form,
// `std_types::text::extend_text`'s, its callback said to run in its call
// alone, so tidy reuses a binding it captures past it (#1168): the
// dictionary's `extend` made its own beside it.
test("a String's stepped extend is extend_text's alone", () => {
  const making = files("src/lower").filter((file) => file.endsWith(".rs") && read(file).includes('"$extendText"'));
  expect(making).toEqual(["src/lower/std_types/text.rs"]);
});
