// serde's built crates as compiles read them, a snapshot of what Cargo
// built (#1711): two runs setting serde up at once each find the snapshot
// whole, and a rebuild of what Cargo built, its files written anew, takes
// none from under them.
import { expect, test } from "bun:test";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { fixture, root } from "./support";
import { copied, keptSerdeFlags, readerOf, serdeSnapshots, sweepSerdeKept } from "./serde-kept";

const hour = 3600 * 1000;

test("two runs setting serde up at once each compile against a whole snapshot, which a rebuild after takes nothing from", async () => {
  const dir = fixture("serde-kept");
  const built = join(dir, "target", "serde");
  const deps = join(built, "deps");
  mkdirSync(join(deps, "out"), { recursive: true });
  const serde = join(built, "libserde-1.rmeta");
  writeFileSync(serde, "serde's metadata ".repeat(20_000));
  writeFileSync(join(deps, "libserde_core-2.rmeta"), "serde_core's");
  writeFileSync(join(deps, "out", "private.rs"), "a build script's");
  const flags = ["--extern", `serde=${serde}`, "-L", `dependency=${deps}`];
  const store = join(dir, "kept");
  const child = join(dir, "setup.ts");
  writeFileSync(
    child,
    `import { keptSerdeFlags } from ${JSON.stringify(join(root, "test/serde-kept.ts"))};
console.log(JSON.stringify(keptSerdeFlags(${JSON.stringify(flags)}, ${JSON.stringify(store)})));
`,
  );
  // Two at once, as two runs' first serde examples.
  const runs = await Promise.all(
    [0, 1].map(async () => {
      const p = Bun.spawn(["bun", child], { stdout: "pipe", stderr: "pipe" });
      const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]);
      expect(code, err).toBe(0);
      return JSON.parse(out) as string[];
    }),
  );
  expect(runs[0]).toEqual(runs[1]);
  expect(runs[0]).toEqual(keptSerdeFlags(flags, store));
  const kept = runs[0];
  expect(kept[0]).toBe("--extern");
  expect(kept[1]).toMatch(new RegExp(`^serde=${store}/[0-9a-f]{24}/0/libserde-1\\.rmeta$`));
  expect(kept[3]).toMatch(new RegExp(`^dependency=${store}/[0-9a-f]{24}/1/deps$`));
  // What Cargo built, rebuilt: its files gone, then written anew.
  rmSync(built, { recursive: true, force: true });
  const keptSerde = kept[1].slice("serde=".length);
  const keptDeps = kept[3].slice("dependency=".length);
  expect(readFileSync(keptSerde, "utf8")).toBe("serde's metadata ".repeat(20_000));
  expect(readFileSync(join(keptDeps, "libserde_core-2.rmeta"), "utf8")).toBe("serde_core's");
  // Not a subdirectory's, which rustc's search never reads: a build
  // script's scratch, written and removed while another run builds (#2311).
  expect(existsSync(join(keptDeps, "out"))).toBe(false);
  // No half-made snapshot left.
  expect(existsSync(store) && require("node:fs").readdirSync(store).some((name: string) => name.includes(".making-"))).toBe(false);
});

// A directory's file gone between listing it and copying it, as
// proc-macro2's probe files go while another run's Cargo builds, is left
// out, not an ENOENT that fails the run's setup (#2311); a file a flag
// names, gone, still fails it.
test("a file Cargo takes away while serde's build is copied is left out", () => {
  const dir = fixture("serde-kept-gone");
  const deps = join(dir, "deps");
  mkdirSync(deps, { recursive: true });
  for (const name of ["liba-1.rmeta", "libb-2.rmeta", "libc-3.rmeta"]) writeFileSync(join(deps, name), name);
  const to = join(dir, "copy");
  const seen: string[] = [];
  copied(deps, to, (from, into) => {
    seen.push(from);
    // As the first is copied, Cargo removes the second.
    if (seen.length === 1) rmSync(join(deps, "libb-2.rmeta"));
    copyFileSync(from, into);
  });
  expect(seen.length).toBe(3);
  expect(require("node:fs").readdirSync(to).sort()).toEqual(["liba-1.rmeta", "libc-3.rmeta"]);
  const file = join(dir, "libserde-1.rmeta");
  writeFileSync(file, "serde's");
  expect(() => copied(file, join(dir, "kept.rmeta"), (from, into) => (rmSync(from), copyFileSync(from, into)))).toThrow("ENOENT");
});

// A snapshot is of one build of serde, a compiler's mostly, so each
// rebuild of the compiler makes another, and kept a day they were 62 in
// the M5 Pro's main checkout, 4.1 GiB (#2483). One goes an hour after it
// was last asked for, but never while a live process that asked for it
// reads it: each marks its own (`readerOf`), and a mark of a process gone
// goes at the next sweep, as a snapshot half made by one does.
test("a snapshot an hour unasked goes, but not one a live process reads", async () => {
  const store = join(fixture("serde-kept-swept"), "kept");
  const now = Date.now();
  const snapshot = (letter: string, age: number) => {
    const at = join(store, letter.repeat(24));
    mkdirSync(join(at, "0"), { recursive: true });
    writeFileSync(join(at, "0", "libserde-1.rmeta"), letter);
    const then = new Date(now - age);
    utimesSync(at, then, then);
    return at;
  };
  const ended = Bun.spawn(["true"]);
  await ended.exited;
  const old = snapshot("a", 2 * hour);
  const read = snapshot("b", 2 * hour);
  writeFileSync(readerOf(read), "");
  const readOnce = snapshot("c", 2 * hour);
  writeFileSync(readerOf(readOnce, ended.pid), "");
  const fresh = snapshot("d", hour / 2);
  mkdirSync(join(store, `${"e".repeat(24)}.making-${ended.pid}`));
  // A prune may take only what no live process reads.
  expect(serdeSnapshots(store).map(({ path }) => basename(path)).sort()).toEqual([old, readOnce, fresh].map((at) => basename(at)));
  expect(sweepSerdeKept(store, now)).toBe(2);
  expect(readdirSync(store).sort()).toEqual([read, readerOf(read), fresh].map((at) => basename(at)).sort());
});

// The process that asks for a snapshot marks it read before it's made or
// found, so a sweep in another run never takes it from under the compiles
// it gives the flags to.
test("a run marks the snapshot it asks for read", () => {
  const dir = fixture("serde-kept-reader");
  const built = join(dir, "libserde-1.rmeta");
  writeFileSync(built, "serde's");
  const store = join(dir, "kept");
  const [, kept] = keptSerdeFlags(["--extern", `serde=${built}`], store);
  const snapshot = join(store, kept.slice(`serde=${store}/`.length).split("/")[0]);
  expect(existsSync(readerOf(snapshot))).toBe(true);
  // Last asked for two hours ago and swept now, it stays while this run lives.
  const then = new Date(Date.now() - 2 * hour);
  utimesSync(snapshot, then, then);
  expect(sweepSerdeKept(store)).toBe(0);
  expect(readFileSync(kept.slice("serde=".length), "utf8")).toBe("serde's");
});
