import { expect, test } from "bun:test";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, rmSync, symlinkSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
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
    // What's stale, the compiler says (ADR 0375): each as it was written.
    const publish = (values: Record<string, string>, stale: Record<string, string> = {}) => {
      const files = new Map(Object.entries(values).map(([name, text]) => [join(dir, name), Buffer.from(text)]));
      const manifest = { version: 1, input: join(dir, "lib.rs"), output: join(dir, "lib.js"), sources: [], modules: [],
        artifacts: [...files].map(([file, bytes]) => ({ file, hash: fingerprint(bytes) })),
        stale: Object.entries(stale).map(([name, text]) => ({ file: join(dir, name), hash: fingerprint(Buffer.from(text)) })) };
      publishArtifacts(path, manifest, files);
      return manifest;
    };
    publish({ "lib.js": "one", "stale.js": "stale", "edited.js": "original" });
    writeFileSync(join(dir, "edited.js"), "user edit");
    // A link to what this build writes, which a WASI compiler can't see is one.
    symlinkSync(join(dir, "lib.js"), join(dir, "alias.js"));
    const manifest = publish({ "lib.js": "two" }, { "stale.js": "stale", "edited.js": "original", "alias.js": "two" });
    expect(existsSync(join(dir, "stale.js"))).toBe(false);
    expect(readFileSync(join(dir, "edited.js"), "utf8")).toBe("user edit");
    expect(existsSync(join(dir, "alias.js"))).toBe(true);
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
test("the catalog of crates is each one's Cargo.toml, each a binding crate", () => {
  type Manifest = { package: { name: string; metadata?: { "rust-js"?: { bindings?: boolean } } }; lib?: { name?: string }; dependencies?: Record<string, { package?: string }> };
  for (const crate of crates) {
    const manifest = Bun.TOML.parse(readFileSync(join(root, crate.dir, "Cargo.toml"), "utf8")) as Manifest;
    expect([crate.dir, manifest.package.name, manifest.lib?.name]).toEqual([crate.dir, crate.cargo, crate.lib]);
    // rustc checks it, and rust-js reads its metadata, as it says (ADR 0414).
    expect([crate.dir, manifest.package.metadata?.["rust-js"]?.bindings]).toEqual([crate.dir, true]);
    // What it uses is shipped too, and built before it.
    for (const dep of Object.values(manifest.dependencies ?? {})) {
      const used = crates.findIndex((other) => other.cargo === dep.package);
      if (dep.package?.startsWith("rust-js-")) expect([crate.dir, dep.package, used >= 0 && used < crates.indexOf(crate)]).toEqual([crate.dir, dep.package, true]);
    }
  }
  // React's, and what it uses.
  expect([...reactLibs].sort()).toEqual(["js", "react", "webapi"]);
});

// Which compiler a host runs (ADR 0854): `RUST_JS_COMPILER`'s, as it is;
// the app's `@rust-js/native`; or a rust-js checkout's own, built from its
// sources as they are first, by its `build:compiler`, which is Cargo's and
// does nothing when they haven't changed. An app built from a checkout ran
// the binary an earlier `cargo build` left, older than its sources, and
// wrote what they no longer did.
test("a checkout's own compiler is built from its sources before it runs, and no other is", () => {
  const dir = realpathSync(mkdtempSync(join(tmpdir(), "rust-js-checkout-")));
  try {
    const checkout = join(dir, "rust-js");
    const app = join(dir, "app");
    // The stand-in checkout's, not this run's `target`, support.ts's.
    const compiler = join(checkout, "target", "debug", "rust-js");
    const built = relative(checkout, compiler);
    mkdirSync(dirname(compiler), { recursive: true });
    mkdirSync(app);
    writeFileSync(join(app, "package.json"), "{}");
    // What an earlier build left, of sources since changed.
    writeFileSync(compiler, "old");
    writeFileSync(join(checkout, "sources"), "new");
    const workspace = (build: string) => writeFileSync(join(checkout, "package.json"), JSON.stringify({ name: "@rust-js/workspace", scripts: { "build:compiler": build } }));
    // A build as Cargo's: the compiler of the sources as they are, each
    // build counted, and where it ran and what it was given said.
    workspace(`echo built >> builds; printf '%s' "$PWD $CARGO_TARGET_DIR \${RUSTC_BOOTSTRAP-unset}" > given; echo '   Compiling rust-js'; cp sources ${built}`);
    const builds = () => (existsSync(join(checkout, "builds")) ? readFileSync(join(checkout, "builds"), "utf8").split("\n").length - 1 : 0);
    const node = Bun.which("node") ?? process.execPath;
    const find = (env: Record<string, string> = {}, at = checkout) => {
      const { RUST_JS_COMPILER: _, ...inherited } = process.env;
      const script = `import { findCompiler } from ${JSON.stringify(join(root, "tooling/build.js"))};
try { process.stdout.write(await findCompiler(${JSON.stringify(app)}, { checkout: ${JSON.stringify(at)} })); }
catch (error) { process.stderr.write(error.message); process.exitCode = 1; }`;
      // Another target directory, and another `RUSTC_BOOTSTRAP`, an app's
      // or a test's, aren't the checkout's build's.
      const p = Bun.spawnSync([node, "--input-type=module", "-e", script], {
        cwd: app, env: { ...inherited, CARGO_TARGET_DIR: join(app, "target"), RUSTC_BOOTSTRAP: "1", ...env }, stdout: "pipe", stderr: "pipe",
      });
      return { code: p.exitCode, stdout: p.stdout.toString(), stderr: p.stderr.toString() };
    };
    // Built first, in the checkout, into the target directory it's run from,
    // as a shell builds it; what the build says is on stderr, as an editor's
    // check reads Cargo's messages from stdout.
    expect(find()).toEqual({ code: 0, stdout: compiler, stderr: "   Compiling rust-js\n" });
    expect(readFileSync(compiler, "utf8")).toBe("new");
    expect(readFileSync(join(checkout, "given"), "utf8")).toBe(`${checkout} ${join(checkout, "target")} unset`);
    expect(builds()).toBe(1);
    // A build that fails stops the host, with what it said and what to
    // run; the compiler an earlier build left isn't run.
    workspace("echo 'error[E0425]: cannot find value `x` in this scope' >&2; exit 101");
    expect(find()).toEqual({
      code: 1,
      stdout: "",
      stderr: `error[E0425]: cannot find value \`x\` in this scope\nrust-js: building ${compiler} from this checkout's sources failed, as it says above; the one an earlier build left isn't run: fix it, or run \`bun run build:compiler\` in ${checkout}`,
    });
    // `RUST_JS_COMPILER` is run as it is, even the checkout's own build:
    // whoever names one builds it, as the tests and scripts do.
    workspace("echo built >> builds");
    expect(find({ RUST_JS_COMPILER: compiler })).toEqual({ code: 0, stdout: compiler, stderr: "" });
    // An installed `@rust-js/build` is of no checkout: there's nothing to
    // build, and its compiler is a release's.
    expect(find({}, join(app, "node_modules/@rust-js"))).toEqual({ code: 0, stdout: join(app, "node_modules/@rust-js", built), stderr: "" });
    // So is the app's `@rust-js/native`.
    const native = join(app, "node_modules/@rust-js/native");
    mkdirSync(native, { recursive: true });
    writeFileSync(join(native, "package.json"), JSON.stringify({ name: "@rust-js/native" }));
    expect(find()).toEqual({ code: 0, stdout: join(native, "bin/rust-js"), stderr: "" });
    expect(builds()).toBe(1);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});
