import { expect, test } from "bun:test";
import { mkdtempSync, writeFileSync, readFileSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fingerprint, publishArtifacts } from "../tooling/publish.js";
import { crates, reactLibs } from "../tooling/crates.js";
import { readReport } from "../wasm/web/frame-report.js";
import { root } from "./support";

test("WASI host removes only unchanged owned artifacts and validates before publication", () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-publish-"));
  try {
    const path = join(dir, "manifest.json");
    const handwritten = join(dir, "handwritten.js");
    writeFileSync(handwritten, "preserve");
    const publish = (values: Record<string, string>) => {
      const files = new Map(Object.entries(values).map(([name, text]) => [join(dir, name), Buffer.from(text)]));
      const manifest = { version: 1, input: join(dir, "lib.rs"), output: join(dir, "lib.js"), sources: [], modules: [],
        artifacts: [...files].map(([file, bytes]) => ({ file, hash: fingerprint(bytes) })) };
      publishArtifacts(path, manifest, files);
      return manifest;
    };
    publish({ "lib.js": "one", "stale.js": "stale", "edited.js": "original" });
    writeFileSync(join(dir, "edited.js"), "user edit");
    const manifest = publish({ "lib.js": "two" });
    expect(existsSync(join(dir, "stale.js"))).toBe(false);
    expect(readFileSync(join(dir, "edited.js"), "utf8")).toBe("user edit");
    expect(readFileSync(handwritten, "utf8")).toBe("preserve");
    expect(() => publishArtifacts(path, manifest, new Map())).toThrow("Missing");
    expect(readFileSync(join(dir, "lib.js"), "utf8")).toBe("two");
    // A later artifact cannot be staged: earlier staged bytes must not leak.
    expect(() => publish({ "lib.js": "three", "handwritten.js/child": "blocked" })).toThrow();
    expect(readFileSync(join(dir, "lib.js"), "utf8")).toBe("two");
  } finally { rmSync(dir, { recursive: true, force: true }); }
});

test("preview reports reject malformed or ambiguous payloads", () => {
  for (const data of [null, "hello", { run: 1 }, { run: -1, ran: true }, { run: 1, ran: "true" },
    { run: 1, error: {}, ran: true }, { run: 1, tested: { passed: -1, failed: 0, ignored: 0 } }]) {
    expect(readReport({ data })).toBeUndefined();
  }
  for (const data of [{ run: 1, ran: true }, { run: 2, error: "failed" },
    { run: 3, tested: { passed: 3, failed: 0, ignored: 0 } }]) expect(readReport({ data })).toEqual(data);
});

// The crates rust-js ships are listed once, tooling/crates.js, which hosts
// and its packaging read (the second architecture review: eight places
// listed them). Each is as its Cargo.toml says, and the compiler's own list
// of bindings, which can't read the catalog, is the catalog's.
test("the catalog of crates is each one's Cargo.toml, and the compiler's bindings", () => {
  type Manifest = { package: { name: string }; lib?: { name?: string }; dependencies?: Record<string, { package?: string }> };
  for (const crate of crates) {
    const manifest = Bun.TOML.parse(readFileSync(join(root, crate.dir, "Cargo.toml"), "utf8")) as Manifest;
    expect([crate.dir, manifest.package.name, manifest.lib?.name]).toEqual([crate.dir, crate.cargo, crate.lib]);
    // What it uses is shipped too, and built before it.
    for (const dep of Object.values(manifest.dependencies ?? {})) {
      const used = crates.findIndex((other) => other.cargo === dep.package);
      if (dep.package?.startsWith("rust-js-")) expect([crate.dir, dep.package, used >= 0 && used < crates.indexOf(crate)]).toEqual([crate.dir, dep.package, true]);
    }
  }
  const cargo = readFileSync(join(root, "src/cargo.rs"), "utf8");
  const bindings = cargo.match(/const BINDINGS: \[&str; \d+\] = \[([^\]]*)\];/)?.[1].match(/"[^"]+"/g)?.map((name) => JSON.parse(name));
  expect(bindings?.sort()).toEqual(crates.filter((crate) => crate.binding).map((crate) => crate.cargo).sort());
  // React's, and what it uses.
  expect([...reactLibs].sort()).toEqual(["js", "react", "webapi"]);
});
