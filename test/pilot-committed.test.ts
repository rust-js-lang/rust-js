// The pilot (ROADMAP M3.3, `examples/pilot`) as it's committed: its
// manifests, which name rust-js's crates and packages by version, and the JS
// rust-js writes beside its Rust. No server, no Vite and no browser, which
// test/pilot.test.ts runs, so a bless of its JS takes its build alone
// (#2524).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { checkCargo } from "../tooling/cargo.js";
import { buildSerde } from "./serde-crate";
import { buildCompiler, compiler, root } from "./support";
import { checkout, install, keptPilot, writtenJs } from "./pilot";
import { channel } from "../scripts/toolchain.ts";
import { underLoad } from "../scripts/timing";

const pilot = keptPilot();

beforeAll(() => {
  buildCompiler();
  install(pilot);
  // serde and serde_json, fetched as serde/build.sh locks them, which the
  // pilot's lockfile has too: its build is offline.
  buildSerde();
  // The packs' limit, and serde's build, kept (`setupLimit`).
}, underLoad(120_000));

// An app of its own (ROADMAP M3.3), as one outside this repository is: its
// crates and rust-js's packages by version, not a member of its workspace.
// It names a release, which this checkout's crates, the ones the test
// installs, are an upgrade of within its requirements: the pilot is the app
// a release's upgrade is checked with (M4.3).
test("the pilot names rust-js's crates and packages by version, not by a path into this checkout", () => {
  const version = (dir: string) => readFileSync(join(root, dir, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];
  const manifest = JSON.parse(readFileSync(join(checkout, "package.json"), "utf8"));
  const frontend = readFileSync(join(checkout, "frontend", "Cargo.toml"), "utf8");
  // `~0.0.2` takes 0.0.2 up to 0.1.0, as Cargo and npm both read it.
  const minor = (v: string) => v.split(".").slice(0, 2).join(".");
  for (const [dir, crate] of [["builtins", "js"], ["webapi", "webapi"], ["react", "react"]]) {
    const named = manifest.dependencies[`@rust-js/${dir}`];
    expect(frontend).toContain(`${crate} = { package = "rust-js-${dir}", version = "~${named}" }`);
    expect(Bun.semver.order(version(dir), named)).toBeGreaterThanOrEqual(0);
    expect(minor(version(dir))).toBe(minor(named));
  }
  // One release of the compiler's packages, the react crate's too: 0.0.3's,
  // whose helpers were `@rust-js/runtime`, which 0.0.4's upgrade replaces
  // with `@rust-js/std` (#1095).
  const release = manifest.devDependencies["@rust-js/native"];
  expect([manifest.dependencies["@rust-js/runtime"], manifest.dependencies["@rust-js/react"], manifest.devDependencies["@rust-js/build"]]).toEqual([release, release, release]);
  expect(manifest.scripts.build).toBe("rust-js build --package frontend --check && vite build web");
  expect(manifest.scripts.postinstall).toBeUndefined();
  for (const crate of ["frontend", "models", "server"]) {
    expect(readFileSync(join(checkout, crate, "Cargo.toml"), "utf8")).not.toContain('path = "../../');
  }
  const workspaces: string[] = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).workspaces;
  expect(workspaces.filter((dir) => dir.startsWith("examples/pilot"))).toEqual([]);
});

// What's committed is what rust-js writes now: a change to the Rust is
// committed with its JS, as ReScript's projects do. Its build, cold, the
// binding crates first: 12.8 s in CI (nightly 45739620208), and 16 s on the
// 12-core M6 at a load of 21, of a mutant's compiler, new to the kept app;
// within the tests' minute, `underLoad`.
test("the pilot's committed JS is what rust-js writes from its Rust", async () => {
  const committed = writtenJs(checkout);
  expect(committed.size).toBeGreaterThan(0);
  // Built here, as `rust-js build` builds it, so a build that fails fails this.
  const react = JSON.parse(readFileSync(join(pilot.dir, "node_modules", "react", "package.json"), "utf8")).version;
  await checkCargo({ manifestPath: join(pilot.dir, "Cargo.toml"), toolchain: channel, compiler, packageNames: ["frontend"], offline: true, react, inSource: true });
  // Blessed as the corpus is, `BLESS=1`: what's written, committed.
  if (process.env.BLESS) {
    for (const [path, js] of writtenJs(pilot.dir)) writeFileSync(join(checkout, path), js);
    return;
  }
  expect(writtenJs(pilot.dir)).toEqual(committed);
});
