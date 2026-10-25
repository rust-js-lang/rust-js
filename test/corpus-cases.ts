// The corpus's cases and what each expects, read from its `//@` directives,
// the one reading of them, which the corpus and tidy's stages both ask
// (ADR 0088):
//
//   //@ run-pass                   main returns (the default)
//   //@ run-fail: <message>        main panics with exactly this message (\n for a newline)
//   //@ compile-fail: <text>       rust-js rejects it, with this in its first error
//   //@ ignore-rust-js: <reason>   rust-js gets it wrong for now; passing is an error; beside
//                                  run-fail, of one that panics
//   //@ edition: <year>            compiled at this edition, 2024 if it says none; beside one of those
//   //@ native: wasm32             native Rust is `wasm32-wasip1`'s, whose `usize` is rust-js's
//                                  (ADR 0090), not the host's; of a case that runs to its end

import { readdirSync } from "node:fs";
import { join } from "node:path";

import { root } from "./support";

export const corpus = join(root, "test/corpus");

/** Each case's file name, in order. */
export const cases = readdirSync(corpus)
  .filter((f) => f.endsWith(".rs"))
  .sort();

export type Expect =
  | { kind: "run-pass" }
  | { kind: "run-fail"; message: string }
  | { kind: "compile-fail"; text: string }
  | { kind: "ignore-rust-js"; reason: string; message?: string };

export type Directives = Expect & { edition: string; native: "host" | "wasm32"; libraryRefused?: string };

/** What a case's directives say, or the problems with them. */
export function directives(source: string): Directives | string {
  const found: Expect[] = [];
  let edition = "2024";
  let native: "host" | "wasm32" = "host";
  // What a library can't do that a program can, and rust-js refuses it of
  // one built as a library (ADR 0360): the refusal's text.
  let libraryRefused: string | undefined;
  for (const [, line] of source.matchAll(/^\/\/@(.*)$/gm)) {
    const [, name, value] = /^ ([a-z-]+)(?:: (.+))?$/.exec(line) ?? [];
    if (name === "edition" && value && ["2015", "2018", "2021", "2024"].includes(value)) edition = value;
    else if (name === "native" && value === "wasm32") native = "wasm32";
    else if (name === "library-refused" && value) libraryRefused = value;
    else if (name === "run-pass" && value === undefined) found.push({ kind: "run-pass" });
    else if (name === "run-fail" && value) found.push({ kind: "run-fail", message: value.replaceAll("\\n", "\n") });
    else if (name === "compile-fail" && value) found.push({ kind: "compile-fail", text: value });
    else if (name === "ignore-rust-js" && value) found.push({ kind: "ignore-rust-js", reason: value });
    else return `unknown or malformed directive \`//@${line}\``;
  }
  // One rust-js gets wrong may panic: with what, beside why it's wrong.
  const [first, second] = found;
  if (found.length === 2 && first.kind === "ignore-rust-js" && second.kind === "run-fail") {
    found.splice(0, 2, { ...first, message: second.message });
  }
  if (found.length > 1) return "more than one directive";
  const expect = found[0] ?? { kind: "run-pass" };
  if (native === "wasm32" && expect.kind !== "run-pass") return "`native: wasm32` aborts on a panic: only of a case that runs to its end";
  return { ...expect, edition, native, libraryRefused };
}
