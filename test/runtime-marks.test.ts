// The marks the runtime sets on what it makes, which every copy of the
// runtime reads (ADR 1184): declared in `runtime/marks.js` alone, each a
// registry symbol, `Symbol.for("rust-js: ..")`. A bundled library and the
// app that uses it may each hold a copy, and a panic one raises is caught,
// downcast or hooked by the other: a mark one copy made by `Symbol(..)`
// is unseen by the other, a `String` payload read as a `&str`.

import { expect, test } from "bun:test";
import { cpSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { libraryCrate } from "./metamorphic";
import { stdDir } from "./packages-of";
import { compileJs, compileJsLibrary, nativeFlags, runJs, runNative } from "./programs";
import { fixture, root } from "./support";

const runtime = join(root, "crates", "rust-js-program", "src", "runtime");

// Each `Symbol(..)` of the runtime that's one copy's alone, and why: what it
// marks is made and read by one copy, or is waiting on its issue to be one
// every copy reads. This list only shrinks.
const oneCopys: Record<string, string> = {
  "chunks.js $partsOf": "a value's parts, which one copy makes and reads (#2333)",
  "cloning.js $cloneKey": "a value's clone, which one copy makes and reads (#2333)",
  "closing.js $slicedKey": "a slice's view, which one copy makes and reads (#2333)",
  "iter.js $arrayIterKey": "an array's iterator, which one copy makes and reads (#2333)",
  "mut_items.js $itemArray": "an item's place, which one copy makes and reads (#2333)",
  "mut_items.js $itemIndex": "an item's place, which one copy makes and reads (#2333)",
  "rc.js $countedKey": "an `Rc`'s count, which one copy makes and reads (#2333)",
  "slice_ops.js $placeOfPart": "a part's place, which one copy makes and reads (#2333)",
};

test("the runtime's marks are registry symbols, declared in marks.js alone", () => {
  const made: string[] = [];
  const registered: string[] = [];
  for (const file of readdirSync(runtime).filter((name) => name.endsWith(".js"))) {
    for (const line of readFileSync(join(runtime, file), "utf8").split("\n")) {
      const declared = /^const (\$\w+) = (Symbol(?:\.for)?)\(/.exec(line);
      if (declared === null) continue;
      const [, name, how] = declared;
      if (how === "Symbol") made.push(`${file} ${name}`);
      // `Symbol.dispose` where the host has none: the well-known symbol's
      // stand-in, not a mark of rust-js's.
      else if (!(file === "dropping.js" && name === "$disposeKey")) registered.push(`${file} ${name} ${line.includes('"rust-js: ') ? "named" : "unnamed"}`);
    }
  }
  expect(made.sort()).toEqual(Object.keys(oneCopys).sort());
  expect(registered.sort()).toEqual(["marks.js $raisedAt named", "marks.js $stringPayload named"]);
});

test("a panic raised by a library's copy of the runtime is its app's copy's: its payload, a String, downcasts, and where it was raised is read, as native Rust's", () => {
  const dir = fixture("runtime-marks-two-copies");
  const libraryFile = join(dir, "library.rs");
  writeFileSync(
    libraryFile,
    'pub fn formatted(n: u32) -> u32 {\n    if n > 2 {\n        panic!("n = {n}");\n    }\n    n\n}\n\npub fn plain() {\n    panic!("plain");\n}\n',
  );
  const programFile = join(dir, "program.rs");
  writeFileSync(
    programFile,
    `use std::panic::catch_unwind;

fn main() {
    for n in [1, 3] {
        match catch_unwind(|| ${libraryCrate}::formatted(n)) {
            Ok(n) => println!("ok {n}"),
            Err(payload) => println!("{:?} {:?}", payload.downcast_ref::<String>(), payload.downcast_ref::<&str>()),
        }
    }
    let payload = catch_unwind(${libraryCrate}::plain).unwrap_err();
    println!("{:?} {:?}", payload.downcast_ref::<String>(), payload.downcast_ref::<&str>());
    ${libraryCrate}::plain();
}
`,
  );
  const rlib = join(dir, `lib${libraryCrate}.rlib`);
  const built = runSync(["rustc", ...nativeFlags("2024"), "--crate-type=rlib", `--crate-name=${libraryCrate}`, libraryFile, "-o", rlib], root, 120_000);
  expect(built.stderr).toBe("");
  const native = runNative(programFile, dir, "2024", "host", ["--extern", `${libraryCrate}=${rlib}`]);
  if (typeof native === "string") throw new Error(native);
  expect(native.stdout).toBe('ok 1\nSome("n = 3") None\nNone Some("plain")\n');
  // And where its last panic, uncaught, was raised, the library's line.
  expect(native.outcome).toEqual({ panic: "plain", at: expect.stringMatching(/library\.rs:9:5$/) });
  const lib = compileJsLibrary(libraryFile, dir, "2024", libraryCrate);
  if (!("rmeta" in lib)) throw new Error(lib.error);
  const program = compileJs(programFile, dir, "2024", false, {}, ["--dependency", lib.manifest], ["--extern", `${libraryCrate}=${lib.rmeta}`, "-L", dir]);
  if (!("js" in program)) throw new Error(program.error);
  // The library's own copy of the runtime, beside it, which Node resolves
  // its `@rust-js/std` to; the app's is the checkout's.
  const copy = join(dir, libraryCrate, "node_modules", "@rust-js", "std");
  mkdirSync(copy, { recursive: true });
  cpSync(stdDir, copy, { recursive: true, dereference: true });
  const ran = runJs(["node"], program.js, dir, "node");
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});
