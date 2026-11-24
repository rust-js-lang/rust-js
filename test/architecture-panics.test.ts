// How a panic is made and where it says it was raised, split from
// architecture.test.ts by concept (a test file is at most 1500 lines).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// A panic's `Error` is made one way, `Expr::panic` (ADR 0706): five
// places wrote `new Error(..)` each, and `panic!("")` was `new Error("")`.
// It's the runtime's `$panic` or `$stringPanic`, which marks where it was
// raised (ADR 1184).
test("a panic's Error is made in one place", () => {
  const made = /Expr::(?:var|global)\("Error"\)|helper\("\$(?:panic|stringPanic)"/;
  expect(compilerRustFiles().filter(file => made.test(read(file)))).toEqual(["crates/rust-js-program/src/js/make/unwinding.rs"]);
});

/** How many arguments the call whose `(` is at `open` in `text` is given,
 * counting its top-level commas, past strings, template literals and their
 * `${..}`. */
function argumentCount(text: string, open: number): number {
  let depth = 0;
  let commas = 0;
  const templates: number[] = [];
  for (let i = open; i < text.length; i++) {
    const c = text[i];
    if (c === '"' || c === "'") {
      for (i++; i < text.length && text[i] !== c; i++) if (text[i] === "\\") i++;
    } else if (c === "`") {
      templates.push(depth);
      for (i++; i < text.length && text[i] !== "`"; i++) {
        if (text[i] === "\\") i++;
        else if (text[i] === "$" && text[i + 1] === "{") break;
      }
      if (text[i] === "`") templates.pop();
      else depth++, i++;
    } else if (c === "}" && templates.length > 0 && depth === templates[templates.length - 1] + 1) {
      depth--;
      for (i++; i < text.length && text[i] !== "`"; i++) {
        if (text[i] === "\\") i++;
        else if (text[i] === "$" && text[i + 1] === "{") break;
      }
      if (text[i] === "`") templates.pop();
      else depth++, i++;
    } else if (c === "(" || c === "[" || c === "{") depth++;
    else if (c === ")" || c === "]" || c === "}") {
      depth--;
      if (depth === 0) return commas + 1;
    } else if (c === "," && depth === 1) commas++;
  }
  throw new Error(`no end to the call at ${open}`);
}

// Each panic the runtime raises says where it was raised, its site, as
// std's hook names it: `$panic(message, raisedAt)`'s or
// `$stringPanic(message, raisedAt)`'s second argument. One std raises
// with no location says so in the comment above it, `// no site: <why>`. A
// collect into an `Rc<[T]>` past a `Layout`'s size raised `<unknown>`
// where native Rust names `rc.rs` (#2489).
test("each panic the runtime raises passes its site, or says why it has none", () => {
  const runtime = files("crates/rust-js-program/src/runtime").filter((file) => file.endsWith(".js"));
  const siteless: string[] = [];
  for (const file of runtime) {
    const text = read(file);
    const lines = text.split("\n");
    for (const call of text.matchAll(/\$(?:stringPanic|panic)\(/g)) {
      const at = text.slice(0, call.index).split("\n").length - 1;
      if (/^\s*(?:\/\/|function )/.test(lines[at])) continue;
      if (argumentCount(text, call.index! + call[0].length - 1) >= 2) continue;
      let above = at - 1;
      while (above >= 0 && /^\s*\/\/ /.test(lines[above]) && !/^\s*\/\/ no site: \S/.test(lines[above])) above--;
      if (above >= 0 && /^\s*\/\/ no site: \S/.test(lines[above])) continue;
      siteless.push(`${file}:${at + 1}`);
    }
  }
  expect(siteless).toEqual([]);
});

// The runtime raises a Rust panic only through `$panic` or `$stringPanic`,
// which mark where std raises it: an `Error` it makes of its own is a
// `rust-js:` bug's, or says in the comment above it why it's no panic, `//
// its own Error: <why>`. A hash table's room past what it counts threw a
// plain `Error`, which std's hook named `<unknown>` (#2487).
test("the runtime makes a panic's Error only through $panic", () => {
  const runtime = files("crates/rust-js-program/src/runtime").filter((file) => file.endsWith(".js"));
  const unsaid: string[] = [];
  for (const file of runtime) {
    const lines = read(file).split("\n");
    lines.forEach((line, at) => {
      if (/^\s*\/\//.test(line) || !/new (?:Error|RangeError|TypeError)\(/.test(line)) return;
      if (/new (?:Error|RangeError|TypeError)\(\s*["'`]rust-js:/.test(line)) return;
      let above = at - 1;
      while (above >= 0 && /^\s*\/\/ /.test(lines[above]) && !/^\s*\/\/ its own Error: \S/.test(lines[above])) above--;
      if (above >= 0 && /^\s*\/\/ its own Error: \S/.test(lines[above])) return;
      unsaid.push(`${file}:${at + 1}`);
    });
  }
  expect(unsaid).toEqual([]);
});
