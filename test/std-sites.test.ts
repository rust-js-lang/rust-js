// The runtime's `$stdSites`, runtime/std_sites.js, where std raises each
// panic std's hook names its own line of, is what the walk of a whole
// std's MIR makes (#2292, ADR 1184): `bun run std` writes it, and one a
// change to the walk would move is caught here.
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, readlinkSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { runSync } from "./child";
import { builtRlibs, linkedSysroot, raisedBuild, stdSites, stdSitesFile } from "../scripts/std";
import { compiler, fixture, root, run } from "./support";
import { underLoad } from "../scripts/timing";

// A path into a machine's own copy of std's source or its home, which no
// JS rust-js writes may hold: std's sites are `/rustc/<commit>/library/..`
// on every machine (ADR 0262's difference 62, #2343).
const local = /\/\.rustup\/|lib\/rustlib\/src\/rust\/|\/Users\/|\/home\//;
const commit = /^commit-hash: (\S+)$/m.exec(run(["rustc", "-vV"]))?.[1];

// Each `.js` file under `dir`, its path.
const jsUnder = (dir: string): string[] =>
  (readdirSync(join(root, dir), { recursive: true }) as string[]).filter((file) => file.endsWith(".js")).map((file) => join(dir, file));

test("no committed JS rust-js wrote names a machine's own source", () => {
  const files = [...jsUnder("std"), ...jsUnder("crates/rust-js-program/src/runtime"), ...jsUnder("test/corpus"), ...jsUnder("test/snapshots"), ...jsUnder("wasm/web/rust")];
  expect(files.length).toBeGreaterThan(3000);
  expect(files.filter((file) => local.test(readFileSync(join(root, file), "utf8")))).toEqual([]);
});

// Each row of `clonedAt` the runtime reads by its key, a step's of
// `$repeating`'s, is one `bun run std` writes: a key the walk names
// otherwise, a catalog path changed, would read `undefined` and raise a
// clone's panic nowhere (#2577).
test("each row of clonedAt the runtime reads by its key is one std's walk writes", () => {
  const sites = readFileSync(join(root, "crates/rust-js-program/src/runtime/std_sites.js"), "utf8");
  const written = new Set(Object.keys(JSON.parse(sites.slice(sites.indexOf("{"), sites.lastIndexOf("}") + 1)).clonedAt));
  const read = jsUnder("crates/rust-js-program/src/runtime")
    .filter((file) => !file.endsWith("std_sites.js"))
    .flatMap((file) => [...readFileSync(join(root, file), "utf8").matchAll(/\$stdSites\.clonedAt\["([^"]+)"\]/g)].map((m) => m[1]));
  expect(read.length).toBeGreaterThan(4);
  expect([...new Set(read)].filter((key) => !written.has(key))).toEqual([]);
});

test("std's bodies name std's files as the shipped std does, wherever its source is", () => {
  const dir = fixture("std-sites-bodies");
  const built = runSync(
    [compiler, "std/lib.rs", "-o", join(dir, "std.js"), "--library", "--manifest", join(dir, "manifest.json"), "--std-bodies", "--", "--crate-name", "rust_js_std"],
    root,
    600_000,
  );
  expect(built.code).toBe(0);
  const iter = readFileSync(join(dir, "iter.js"), "utf8");
  expect(iter).toContain(`"/rustc/${commit}/library/core/src/iter/adapters/step_by.rs:`);
  expect(readdirSync(dir).filter((file) => file.endsWith(".js") && local.test(readFileSync(join(dir, file), "utf8")))).toEqual([]);
}, 600_000);

test("the runtime's sites of std's panics are what std's MIR says", () => {
  const out = join(fixture("std-sites"), "raised.json");
  const built = raisedBuild(compiler, root, out);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  expect(stdSites(readFileSync(out, "utf8"), root)).toBe(readFileSync(stdSitesFile, "utf8"));
}, 300_000);

// A library Cargo puts in two directories, a build step's `out` and `deps`,
// is linked into the sysroot once; two of one name that differ are said
// (run 45092210652's `EEXIST`).
test("std's build's libraries are each linked once, and two of one name that differ are said", () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-rlibs-"));
  try {
    for (const at of ["deps", "build/std_detect/a1/out"]) {
      mkdirSync(join(dir, at), { recursive: true });
      writeFileSync(join(dir, at, "libstd_detect-a1.rlib"), "same");
    }
    const once = builtRlibs(dir);
    expect(typeof once === "string" ? once : [...once.keys()]).toEqual(["libstd_detect-a1.rlib"]);
    writeFileSync(join(dir, "deps", "libstd_detect-a1.rlib"), "other");
    expect(builtRlibs(dir)).toContain("two libraries of one name differ");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// The sysroot is made whole beside where it goes and moved in, so runs at
// once, a mutation run's controls, each find it whole: one removed it as
// another linked into it (`EEXIST`, #2459). Twelve processes at once.
test("std's sysroot made by many runs at once is whole in each", async () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-sysroots-"));
  try {
    const rlibs = new Map<string, string>();
    // Enough that linking them takes longer than the runs take to start.
    const names = ["libcore-a1.rlib", "liballoc-b2.rlib", "libstd-c3.rlib", ...Array.from({ length: 3000 }, (_, i) => `libdep${i}-d4.rlib`)];
    for (const name of names) {
      writeFileSync(join(dir, name), name);
      rlibs.set(name, join(dir, name));
    }
    // Each says the sysroot it was given and how many libraries it held
    // then, as a reader finds it.
    const script = [
      `import { readdirSync } from "node:fs";`,
      `import { linkedSysroot } from ${JSON.stringify(join(root, "scripts", "std.ts"))};`,
      `const sysroot = linkedSysroot(${JSON.stringify(dir)}, "wasm32-unknown-unknown", new Map(${JSON.stringify([...rlibs])}));`,
      `console.log(sysroot + " " + readdirSync(sysroot + "/lib/rustlib/wasm32-unknown-unknown/lib").length);`,
    ].join("\n");
    writeFileSync(join(dir, "make.ts"), script);
    const runs = await Promise.all(
      Array.from({ length: 12 }, async () => {
        const p = Bun.spawn([process.execPath, join(dir, "make.ts")], { stdout: "pipe", stderr: "pipe" });
        const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]);
        return { out: out.trim(), err, code };
      }),
    );
    expect(runs.filter((r) => r.code !== 0).map((r) => r.err)).toEqual([]);
    const sysroot = runs[0]!.out.split(" ")[0]!;
    expect(new Set(runs.map((r) => r.out))).toEqual(new Set([`${sysroot} ${names.length}`]));
    expect(sysroot).toBe(linkedSysroot(dir, "wasm32-unknown-unknown", rlibs));
    const lib = join(sysroot, "lib", "rustlib", "wasm32-unknown-unknown", "lib");
    expect(readdirSync(lib).sort()).toEqual([...names].sort());
    expect(readlinkSync(join(lib, "libstd-c3.rlib"))).toBe(join(dir, "libstd-c3.rlib"));
    expect(readdirSync(dir).filter((name) => name.startsWith(".sysroot-"))).toEqual([]);
    expect(existsSync(join(dir, "sysroot"))).toBe(false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}, underLoad(60_000));
