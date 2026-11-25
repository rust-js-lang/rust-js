// The cleanup rules of the architecture, split from architecture.test.ts at
// its cap (a test file is at most 1500 lines): a `catch` that runs a drop
// as a panic unwinds is guarded by one owner, `drops::cleanup` (#847, #938).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root, stdDir } from "./support";
import { compilerRustFiles } from "../scripts/compiler-crates";

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
    "crates/rust-js-lower/src/drops/parts.rs": "drop glue's, its cleanup `cleanup`'s where it's made; a body's `dropped_after`, which tidy guards",
    "crates/rust-js-mir/src/drops.rs": "the MIR's cleanups, a function body's, which tidy's last pass guards",
    "crates/rust-js-mir/src/writer.rs": "a writer's `Err` and what it wrote, no drop",
    "crates/rust-js-mir/src/std_calls.rs": "what a write wrote before it failed, no drop",
    "crates/rust-js-lower/src/format_args.rs": "a formatting failure caught, no drop",
    "crates/rust-js-lower/src/display/printing.rs": "a formatting failure caught as a line is printed through stdout's, no drop (#1490)",
    "crates/rust-js-mir/src/given_arguments.rs": "what an `Arguments` given on made before its writer failed, no drop (#1565)",
    "crates/rust-js-mir/src/lib.rs": "a panic out of a function whose ABI can't unwind, `extern \"C\"`'s, aborted, no drop (#1582)",
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
  const found = [...files("crates/rust-js-lower/src"), ...files("crates/rust-js-mir/src"), "src/pipeline.rs", ...files("src/pipeline")]
    .filter((file) => file.endsWith(".rs") && !file.startsWith("crates/rust-js-mir/src/tidy"))
    .filter((file) => constructions(read(file)) > 0);
  expect([...found].sort()).toEqual(Object.keys(made).sort());
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

// `std_call_never_unwinds` reads std's MIR for prepared.rs; it is no oracle
// of whether the crate's code unwinds. Asked beside `std_never_unwinds` in
// the unwinding judges, 42 corpus cases failed, never_unwinding_calls.rs
// dropping a `drop c` native Rust prints (#1930). Only what it was written
// for calls it.
test("no unwinding judge asks std_call_never_unwinds", () => {
  const uncommented = (path: string) => read(path).replace(/^\s*\/\/.*$/gm, "");
  const callers = compilerRustFiles()
    .filter((file) => file.endsWith(".rs"))
    .filter((file) => /(?<!fn )\bstd_call_never_unwinds\s*\(/.test(uncommented(file)));
  expect(callers).toEqual(["crates/rust-js-lower/src/analysis/prepared.rs"]);
});

// "Is a type `TrustedLen`?" has one answerer, recognition's
// `trusted_len_in` (`Recognition::trusted_len`), its tri-state each
// caller's to decide, and `may_be_trusted_len` beside it (#2170): no
// other looks up `TypeFact::TrustedLen`'s trait to select it, and
// `is_trusted_len`, the second answer, is gone.
test("one answerer of whether a type is TrustedLen", () => {
  const uncommented = (path: string) => read(path).replace(/^\s*\/\/.*$/gm, "");
  const files = compilerRustFiles().filter((file) => file.endsWith(".rs"));
  const lookups = files.filter((file) =>
    /fact_trait\(\s*[^,()]*,\s*(?:crate::recognition::)?TypeFact::TrustedLen\s*\)/.test(uncommented(file)),
  );
  expect(lookups).toEqual(["crates/rust-js-lower/src/recognition/specialization.rs"]);
  expect(files.filter((file) => /\bis_trusted_len\b/.test(read(file)))).toEqual([]);
});
