// The corpus's cases and what each expects, read from its `//@` directives,
// the one reading of them, which the corpus and tidy's stages both ask
// (ADR 0088):
//
//   //@ run-pass                   main returns (the default)
//   //@ run-fail: <message>        main panics with exactly this message (\n for a newline)
//   //@ run-abort                  the process aborts: a drop panicked while a panic unwound (#847)
//   //@ compile-fail: <text>       rust-js rejects it, with this in its first error
//   //@ ignore-rust-js: <reason>   rust-js gets it wrong for now; passing is an error; beside
//                                  run-fail, of one that panics
//   //@ edition: <year>            compiled at this edition, 2024 if it says none; beside one of those
//   //@ native: wasm32             native Rust is `wasm32-wasip1`'s, whose `usize` is rust-js's
//                                  (ADR 0090), not the host's; of a case that runs to its end
//   //@ library-differs            its JS built as a library isn't its program's, so each push
//                                  builds it as one too (ADR 0360); `bun run bless` writes it
//   //@ library-refused: <text>    rust-js refuses it built as a library, with this in its error
//   //@ instances-keep-cost: <why> the metamorphic net's `generic-instances` gives each place an
//                                  instance that answers it without stepping each item where the
//                                  case's own may, its point what its iterators skip at once (#1109)
//   //@ nondeterministic: <why>    its run reads the clock, randomness, the file system or its
//                                  environment, so native Rust's run isn't kept by what it was
//                                  built from, but run each time (test/native-outcomes.ts)
//   //@ located-otherwise: #<issue> <why>
//                                  where its panic is raised isn't native's until the issue is
//                                  fixed (#2292): its runs compared but for that; one that's
//                                  native's is an error; beside run-fail

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
  | { kind: "run-abort" }
  | { kind: "compile-fail"; text: string }
  | { kind: "ignore-rust-js"; reason: string; message?: string };

export type Directives = Expect & {
  locatedOtherwise?: string;
  edition: string;
  native: "host" | "wasm32";
  libraryDiffers: boolean;
  libraryRefused?: string;
  instancesKeepCost?: string;
  nondeterministic?: string;
};

/** Why a program's run reads the world, the clock, randomness, the file
 * system or its environment, as its `//@ nondeterministic: <why>` says, of
 * a corpus case or a crate of a chain (test/crates.ts): its native run
 * isn't kept (test/native-outcomes.ts). */
export function nondeterministic(source: string): string | undefined {
  return /^\/\/@ nondeterministic: (.+)$/m.exec(source)?.[1];
}

/** The issue and why a case's panic is raised otherwise than native's, as
 * its `//@ located-otherwise: #<issue> <why>` says (#2292), or the guide's
 * difference it is, an ADR 0262 exception, `difference <n>, <why>`
 * (#2509): every net
 * compares its runs but for where it was raised (`runNative`'s), and the
 * corpus that it still differs. */
export function locatedOtherwise(source: string): string | undefined {
  return /^\/\/@ located-otherwise: ((?:#\d+|difference \d+,?) .+)$/m.exec(source)?.[1];
}

/** What a case's directives say, or the problems with them. */
export function directives(source: string): Directives | string {
  const found: Expect[] = [];
  let edition = "2024";
  let native: "host" | "wasm32" = "host";
  // What a library can't do that a program can, and rust-js refuses it of
  // one built as a library (ADR 0360): the refusal's text.
  let libraryRefused: string | undefined;
  let libraryDiffers = false;
  // Why the metamorphic net's instances keep each place's cost (#1109).
  let instancesKeepCost: string | undefined;
  // Why its native run isn't kept (`nondeterministic`).
  let why: string | undefined;
  // Why where it panics isn't native's, and its issue (`locatedOtherwise`).
  let located: string | undefined;
  for (const [, line] of source.matchAll(/^\/\/@(.*)$/gm)) {
    const [, name, value] = /^ ([a-z-]+)(?:: (.+))?$/.exec(line) ?? [];
    if (name === "edition" && value && ["2015", "2018", "2021", "2024"].includes(value)) edition = value;
    else if (name === "native" && value === "wasm32") native = "wasm32";
    else if (name === "library-refused" && value) libraryRefused = value;
    else if (name === "library-differs" && value === undefined) libraryDiffers = true;
    else if (name === "instances-keep-cost" && value) instancesKeepCost = value;
    else if (name === "nondeterministic" && value) why = value;
    else if (name === "located-otherwise" && value && /^(?:#\d+|difference \d+,?) ./.test(value)) located = value;
    else if (name === "run-pass" && value === undefined) found.push({ kind: "run-pass" });
    else if (name === "run-fail" && value) found.push({ kind: "run-fail", message: value.replaceAll("\\n", "\n") });
    else if (name === "run-abort" && value === undefined) found.push({ kind: "run-abort" });
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
  if (located !== undefined && expect.kind !== "run-fail") return "`located-otherwise` is of a case that panics: beside `run-fail`";
  return { ...expect, locatedOtherwise: located, edition, native, libraryDiffers, libraryRefused, instancesKeepCost, nondeterministic: why };
}
