// The cleanup rules of the architecture, split from architecture.test.ts at
// its cap (a test file is at most 1500 lines): a `catch` that runs a drop
// as a panic unwinds is guarded by one owner, `drops::cleanup` (#847, #938).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root, stdDir } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);

// A `catch` that runs a drop as a panic unwinds guards it, `$cleanup(() =>
// { .. })`, where a throw, a drop's panic as that panic unwinds, aborts as
// Rust's does (#847). The guard is `drops::cleanup`'s. A catch made outside
// a function's body, a closure's drop of its captures or a std struct's
// `close`, never passes through tidy's last pass, so drop glue's is guarded
// where it's made, `drops/parts.rs`'s `dropped_after_unwinding` (#938). A
// body's `dropped_after` and the MIR's own cleanups, `mir/drops.rs`'s, are
// a function body's, which tidy's last pass guards by the same owner; tidy
// (`mir/tidy`) only rewrites catches it was given. Every other catch made
// runs no drop. So a new one is placed here, its reason with it.
test("a catch that runs a drop guards it where it's made, or is a body's that tidy guards (#938)", () => {
  const made: Record<string, string> = {
    "src/lower/drops/parts.rs": "drop glue's, its cleanup `cleanup`'s where it's made; a body's `dropped_after`, which tidy guards",
    "src/lower/mir/drops.rs": "the MIR's cleanups, a function body's, which tidy's last pass guards",
    "src/lower/mir/writer.rs": "a writer's `Err` and what it wrote, no drop",
    "src/lower/mir/std_calls.rs": "what a write wrote before it failed, no drop",
    "src/lower/format_args.rs": "a formatting failure caught, no drop",
    "src/lower/display/printing.rs": "a formatting failure caught as a line is printed through stdout's, no drop (#1490)",
  };
  // `StmtKind::TryCatch(..)` made, `.at(..)` after its arguments: not a
  // pattern, which never is.
  const constructions = (text: string) => {
    let count = 0;
    for (const m of text.matchAll(/StmtKind::TryCatch\(/g)) {
      let depth = 1;
      let at = m.index! + m[0].length;
      for (; depth > 0 && at < text.length; at++) depth += text[at] === "(" ? 1 : text[at] === ")" ? -1 : 0;
      if (/^\s*\.at\(/.test(text.slice(at, at + 40))) count++;
    }
    return count;
  };
  const found = ["src/lower.rs", ...files("src/lower")]
    .filter((file) => file.endsWith(".rs") && !file.startsWith("src/lower/mir/tidy"))
    .filter((file) => constructions(read(file)) > 0);
  expect(found).toEqual(Object.keys(made).sort());
  expect(constructions("let s = StmtKind::TryCatch(body, Some(e), h).at(span);")).toBe(1);
  expect(constructions("StmtKind::TryCatch(inner, Some(error), handler) if ok => {}")).toBe(0);
  // What std's bodies compile to: each catch that throws on what it caught
  // runs only `$cleanup(..)` before it, std's structs' `close` and its
  // closures' drops among them.
  const unguarded = (js: string) =>
    [...js.matchAll(/catch \((\w+)\) \{\n/g)].flatMap((m) => {
      let depth = 1;
      let at = m.index! + m[0].length;
      for (; depth > 0 && at < js.length; at++) depth += js[at] === "{" ? 1 : js[at] === "}" ? -1 : 0;
      const handler = js.slice(m.index! + m[0].length, at - 1).trim();
      const rethrow = `throw ${m[1]};`;
      if (!handler.endsWith(rethrow)) return [];
      const before = handler.slice(0, -rethrow.length).trim();
      return before === "" || before.startsWith("$cleanup(") ? [] : [before];
    });
  const std = readFileSync(join(stdDir, "iter.js"), "utf8");
  expect(std.match(/catch \(\w+\) \{/g)!.length).toBeGreaterThan(20);
  expect(unguarded(std)).toEqual([]);
  expect(unguarded("} catch (error) {\n  dropF?.(f);\n  throw error;\n}\n")).toEqual(["dropF?.(f);"]);
  expect(unguarded("} catch (error) {\n  if (p) {\n    drop(p);\n  }\n  throw error;\n}\n")).toHaveLength(1);
  expect(unguarded("} catch (error) {\n  $cleanup(() => {\n    drop(p);\n  });\n  throw error;\n}\n")).toEqual([]);
});
