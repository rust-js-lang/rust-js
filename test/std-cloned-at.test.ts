// Where std raises a `RefCell`'s panic, cloned mutably borrowed by a std
// type's `clone`, of the types whose clone reaches it by more than one path
// in std's MIR, so a walk can't say which one runs (#2509): run natively,
// test/std-cloned-at.rs's probe, and kept as the oracle
// crates/rust-js-lower/src/recognition/compiled_std/cloned_at_oracle.txt,
// which `bun run std`'s `clonedAt` reads where its walk finds several, and
// refuses a type that has none of. A toolchain whose std raises another
// fails here; `BLESS=1` writes the oracle again, whose diff is std's.

import { expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { underLoad } from "../scripts/timing";
import { contentDirectory, nativeBinary, root } from "./support";

const oracleFile = join(root, "crates/rust-js-lower/src/recognition/compiled_std/cloned_at_oracle.txt");
const probeFile = join(root, "test/std-cloned-at.rs");

/** The oracle: a heading, then each line the probe printed, its file
 * from std's `library/`, as `bun run std`'s walk names it. */
function oracle(): string {
  const source = readFileSync(probeFile, "utf8");
  const built = nativeBinary(source, contentDirectory(source), ["--edition=2024", "-Awarnings"]);
  if ("error" in built) throw new Error(built.error);
  const ran = Bun.spawnSync([built.binary], { timeout: 30_000, maxBuffer: 1 << 20 });
  if (ran.exitCode !== 0) throw new Error(`the probe failed: ${ran.stderr.toString()}`);
  const lines = ran.stdout.toString().trimEnd().split("\n").map((line) => {
    const at = line.indexOf("library/");
    if (at < 0) throw new Error(`a site outside std's library/: ${line}`);
    return `${line.slice(0, line.lastIndexOf(" "))} ${line.slice(at)}`;
  });
  return [
    "# Where std raises a `RefCell`'s panic as a std type's `clone` clones it, of the",
    "# types whose clone reaches it by more than one path in std's MIR (#2509), made",
    "# by test/std-cloned-at.test.ts from this toolchain's std run natively",
    "# (`BLESS=1 bun test test/std-cloned-at.test.ts` writes it; never by hand).",
    "# `<type> <its type parameter's position> <site>`.",
    ...lines,
    "",
  ].join("\n");
}

test("the oracle of where std's clones raise is what std's runs do", () => {
  const made = oracle();
  if (process.env.BLESS) writeFileSync(oracleFile, made);
  expect(existsSync(oracleFile)).toBe(true);
  expect(readFileSync(oracleFile, "utf8")).toBe(made);
}, underLoad(120_000));
