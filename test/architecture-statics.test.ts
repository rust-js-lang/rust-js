// Each `static` and `thread_local!` of the compiler, and what it holds
// between the builds one process compiles (#2301, ADR 1182): the worker
// compiles a case's program and library builds in one process, so what a
// build leaves in one is what the next starts with. Each is one of these
// classes, which say why no build reads another's; a new one says which
// it is here, or the worker's builds could differ from their own
// compiles. (A rustc session's ids are never held in one but the lowering
// session's caches: architecture.test.ts.)
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { compilerRustFiles } from "../scripts/compiler-crates";
import { root } from "./support";

type Class =
  // The same in every compile: a table parsed once, rustc's own providers
  // kept to call, a constant made once.
  | "constant"
  // Of one compile, one rustc session or one lowering session: set as it
  // starts, or scoped to it, before anything reads it.
  | "compile"
  // Set by a flag of rust-js's, which the worker refuses to share a session
  // of (`worker.rs`'s `shared_part`): false, or empty, in each of its builds.
  | "flag"
  // What a flag's report gathers, read only where its flag asks for it.
  | "report"
  // The process's own, written once as it ends: a decisions' count, which
  // counts each build of a job as their processes' lines would, summed.
  | "process"
  // A name's fresh id, or a name made once of its word: compared only for
  // equality, never printed nor ordered by, so one made by an earlier
  // build is one, as the build's own would be (`test/worker.test.ts`
  // compiles a job's builds in both orders).
  | "identity";

const statics: Record<string, Class> = {
  // The compiler's build, `build.rs`'s hash, given once as main starts
  // (ADR 1181): the same in every build a process compiles.
  "crates/rust-js-program/src/manifest.rs BUILD": "constant",
  "crates/rust-js-program/src/runtime.rs SITES": "constant",
  "crates/rust-js-program/src/decisions.rs PATH": "process",
  "crates/rust-js-program/src/decisions.rs GIVEN": "process",
  "crates/rust-js-program/src/decisions.rs BODY": "process",
  "crates/rust-js-jsx/src/lib.rs CRATE_ID": "compile",
  "src/main.rs RUSTC_TOOLS": "constant",
  "src/phase_times.rs TIMES": "compile",
  "crates/rust-js-program/src/prepare/passes.rs RUNNING": "compile",
  "crates/rust-js-program/src/js/make.rs UNDEFINED": "constant",
  "crates/rust-js-program/src/js/make.rs RAISED_AT": "compile",
  "crates/rust-js-program/src/js/known.rs FORGOTTEN": "flag",
  "crates/rust-js-program/src/js/known.rs FOUND": "report",
  "crates/rust-js-program/src/js/storage.rs KEYS": "constant",
  "crates/rust-js-program/src/js/name.rs NEXT": "identity",
  "crates/rust-js-program/src/js/validate.rs ON": "constant",
  "crates/rust-js-program/src/runtime/signatures.rs DECLARED": "constant",
  "crates/rust-js-jsx/src/resolved.rs STATE": "compile",
  "crates/rust-js-jsx/src/resolved.rs HOLD": "compile",
  "crates/rust-js-lower/src/session.rs SESSION": "compile",
  "crates/rust-js-lower/src/liveness.rs RUSTC_LIVENESS": "constant",
  "crates/rust-js-lower/src/liveness.rs RUSTC_BODY_LIVENESS": "constant",
  "crates/rust-js-lower/src/recognition/std_paths.rs FIRST": "flag",
  "crates/rust-js-lower/src/recognition/std_steps.rs PARSED": "constant",
  "crates/rust-js-lower/src/session/label_census.rs RECORDS": "flag",
  "crates/rust-js-lower/src/session/label_census.rs CENSUS": "report",
  "crates/rust-js-lower/src/session.rs NO_THIR_HINTS": "flag",
  "crates/rust-js-mir/src/tidy.rs MADE": "identity",
  "crates/rust-js-lower/src/session/tidy_fired.rs RECORDS": "flag",
  "crates/rust-js-lower/src/session/tidy_fired.rs FIRED": "report",
  "crates/rust-js-mir/src/tidy/moves.rs T1": "identity",
};

/** Each `static` of `text`, an item's or a `thread_local!`'s, by its name. */
export function declaredStatics(text: string): string[] {
  return [...text.matchAll(/^\s*(?:pub(?:\([^)]*\))?\s+)?static\s+([A-Z_][A-Z0-9_]*)\s*:/gm)].map((m) => m[1]);
}

test("each static of the compiler says what it holds between a process's builds", () => {
  // Every crate's of the compiler's (scripts/compiler-crates.ts), which
  // the test map reads as this test reading them all.
  const found = compilerRustFiles().flatMap((path) => declaredStatics(readFileSync(join(root, path), "utf8")).map((name) => `${path} ${name}`));
  const unsaid = found.filter((key) => !(key in statics));
  expect(unsaid.map((key) => `${key}: say its class in test/architecture-statics.test.ts`)).toEqual([]);
  // And none is listed that's gone.
  expect(Object.keys(statics).filter((key) => !found.includes(key))).toEqual([]);
});

test("the statics are found as a thread-local's and an item's are written", () => {
  expect(declaredStatics("thread_local! {\n    static MADE: RefCell<u8> = Default::default();\n}\npub(crate) static NEXT: u8 = 0;\n")).toEqual(["MADE", "NEXT"]);
});
