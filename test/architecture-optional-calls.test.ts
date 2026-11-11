// JS's optional call, `f?.(arg)`, evaluates no argument where `f` is
// `undefined`: `drop?.($entryInsert(entry._0, value))` wrote nothing of a
// map whose values have no destructor, as no `drop` was given (#1369). So an
// optional call's arguments are reads that do nothing, a name or a member
// read of one: in the runtime, each one written so, a call or anything
// else named first, `const old = ..; drop?.(old)`; in the compiler, each
// made by `Expr::optional_call` (src/js/optional_call.rs), the one maker, which writes
// `f === undefined ? arg : f(arg)` of an argument that does something.
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
const files = (directory: string, ending: string): string[] =>
  readdirSync(join(root, directory), { withFileTypes: true })
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
    .flatMap((entry) =>
      entry.isDirectory()
        ? files(join(directory, entry.name), ending)
        : entry.name.endsWith(ending)
          ? [relative(root, join(root, directory, entry.name))]
          : [],
    );

/** Each optional call's arguments in `text`, its line and their text, the
 * parentheses after `?.(` balanced. */
function optionalCalls(text: string): { line: number; args: string }[] {
  const found: { line: number; args: string }[] = [];
  let at = text.indexOf("?.(");
  while (at >= 0) {
    let depth = 1;
    let end = at + 3;
    while (end < text.length && depth > 0) {
      if (text[end] === "(") depth++;
      else if (text[end] === ")") depth--;
      end++;
    }
    found.push({ line: text.slice(0, at).split("\n").length, args: text.slice(at + 3, end - 1) });
    at = text.indexOf("?.(", end);
  }
  return found;
}

/** Whether `arg` is a read that does nothing: a name, `undefined`, a
 * number, or a member read of a name, `entry.error`, `items[0]`, `v[i]`. */
const plainRead = (arg: string) => /^([\w$]+)(\.[\w$]+|\[(\d+|[\w$]+)\])*$/.test(arg.trim());

/** `args`, split at their top-level commas. */
function split(args: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let from = 0;
  for (let i = 0; i < args.length; i++) {
    if ("([{".includes(args[i])) depth++;
    else if (")]}".includes(args[i])) depth--;
    else if (args[i] === "," && depth === 0) {
      parts.push(args.slice(from, i));
      from = i + 1;
    }
  }
  parts.push(args.slice(from));
  return parts.filter((part) => part.trim() !== "");
}

test("each optional call of the runtime is given reads that do nothing", () => {
  const offending: string[] = [];
  for (const file of files("src/runtime", ".js")) {
    // Its code, each line's comment blanked, so a comment's `?.(..)` isn't one.
    const code = read(file).replace(/\/\/.*$/gm, "");
    for (const { line, args } of optionalCalls(code)) {
      for (const arg of split(args)) {
        if (!plainRead(arg)) offending.push(`${file}:${line}: \`?.(${args})\`, whose \`${arg.trim()}\` isn't evaluated where the callee is none: name it first`);
      }
    }
  }
  expect(offending).toEqual([]);
});

test("the compiler makes an optional call by Expr::optional_call alone", () => {
  const makers = files("src", ".rs").filter(
    (file) => file !== "src/js/optional_call.rs" && /ExprKind::OptionalCall\(\s*Box::new/.test(read(file)),
  );
  expect(makers).toEqual([]);
  expect(read("src/js/optional_call.rs")).toContain("pub fn optional_call(callee: Expr, args: Vec<Expr>) -> Expr {");
});
