import { expect, test } from "bun:test";
import { cpSync, mkdtempSync, mkdirSync, chmodSync, realpathSync, readFileSync, readdirSync, existsSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { crates } from "../tooling/crates.js";
import { buildCompiler, compiler, root as repository } from "./support";

const node = Bun.which("node");
// A distribution to test as it is, as qualification gives one (ADR 0094);
// else one is packed here.
const supplied = process.env.RUST_JS_DISTRIBUTION;
if (!node) throw new Error("Node.js is required for distribution runtime compatibility tests");

// Node, the runtime rust-js targets (ADR 0095), with Bun out of reach.
for (const runtime of [node]) {
test(`installed packages compile using ${runtime} without the other runtime`, () => {
  buildCompiler();
  const root = realpathSync(mkdtempSync(join(tmpdir(), "rust-js packages ")));
  const blocked = join(root, "blocked-runtime");
  mkdirSync(blocked);
  const other = join(blocked, runtime === node ? "bun" : "node");
  writeFileSync(other, "#!/bin/sh\necho 'Unexpected dependency on the other JS runtime' >&2\nexit 99\n");
  chmodSync(other, 0o755);
  const run = (args: string[], cwd = root) => {
    // An empty cache of its own, as a new machine has: a warm one hid that an
    // offline install needed Vite's registry entry.
    const env = { ...process.env, PATH: `${blocked}:${process.env.PATH}`, BUN_INSTALL_CACHE_DIR: join(root, "bun-cache") };
    const result = Bun.spawnSync(args, { cwd, stdout: "pipe", stderr: "pipe", env });
    if (result.exitCode !== 0) throw new Error(result.stderr.toString());
    return result.stdout.toString();
  };
  try {
    const bundle = join(root, "artifacts");
    const pack = [process.execPath, join(repository, "scripts/package-distribution.ts"), compiler, bundle];
    // A copy of one supplied, as this damages one of its files.
    if (supplied) cpSync(supplied, bundle, { recursive: true });
    else run(pack);
    const distribution = JSON.parse(readFileSync(join(bundle, "distribution.json"), "utf8"));
    expect(distribution.compiler).toEqual(JSON.parse(run([compiler, "--version-json"])));
    // The packages, and the crates an app's Cargo builds with.
    expect(distribution.artifacts.map(({ file }: { file: string }) => file).sort()).toEqual(
      ["build.tgz", "native.tgz", "next-plugin.tgz", "runtime.tgz", "vite-plugin.tgz", ...crates.map(({ dir }) => `${dir}.tgz`)].sort(),
    );
    run(["shasum", "-a", "256", "-c", "SHA256SUMS"], bundle);
    const previousDistribution = readFileSync(join(bundle, "distribution.json"), "utf8");
    expect(() => run(pack)).toThrow("Output already exists");
    expect(readFileSync(join(bundle, "distribution.json"), "utf8")).toBe(previousDistribution);
    const incompatible = join(root, "incompatible-compiler");
    writeFileSync(incompatible, `#!/bin/sh\ncat <<'IDENTITY'\n${JSON.stringify({ ...distribution.compiler, toolchain: "nightly-2000-01-01" })}\nIDENTITY\n`);
    chmodSync(incompatible, 0o755);
    const failedBundle = join(root, "failed-artifacts");
    expect(() => run([process.execPath, join(repository, "scripts/package-distribution.ts"), incompatible, failedBundle])).toThrow("Compiler does not match");
    expect(existsSync(failedBundle)).toBe(false);
    expect(readdirSync(root).some(name => name.startsWith(".rust-js-distribution-"))).toBe(false);
    const crateVersion = (dir: string) => (Bun.TOML.parse(readFileSync(join(repository, dir, "Cargo.toml"), "utf8")) as { package: { version: string } }).package.version;
    writeFileSync(join(root, "package.json"), JSON.stringify({
      private: true, type: "module", dependencies: {
        "@rust-js/build": "./artifacts/build.tgz",
        "@rust-js/vite-plugin": "./artifacts/vite-plugin.tgz",
        "@rust-js/native": "./artifacts/native.tgz",
        "@rust-js/runtime": "./artifacts/runtime.tgz",
      },
      overrides: { "@rust-js/build": "./artifacts/build.tgz" },
    }));
    // No registry access or lifecycle scripts. Real Vite is exercised by
    // vite.test.ts; this test invokes its plugin hooks without the peer.
    const install = [process.execPath, "install", "--offline", "--ignore-scripts", "--omit", "peer", "--backend", "copyfile"];
    run(install);
    run([...install, "--frozen-lockfile"]);
    // React's crate, and those it uses, the distribution's, as a package
    // manager installs them: their peers, React's, aren't this test's.
    for (const dir of ["builtins", "webapi", "react"]) {
      const at = join(root, "node_modules", "@rust-js", dir);
      mkdirSync(at, { recursive: true });
      run(["tar", "-xzf", join(bundle, `${dir}.tgz`), "-C", at, "--strip-components", "1"]);
    }
    // Named by the app as a created app names them, which is how the
    // plugin's patch finds them.
    const app = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
    for (const dir of ["builtins", "webapi", "react"]) app.dependencies[`@rust-js/${dir}`] = crateVersion(dir);
    writeFileSync(join(root, "package.json"), JSON.stringify(app));
    const plugin = JSON.parse(readFileSync(join(root, "node_modules/@rust-js/vite-plugin/package.json"), "utf8"));
    expect(plugin.dependencies["@rust-js/build"]).toBe((Bun.TOML.parse(readFileSync(join(repository, "Cargo.toml"), "utf8")) as { package: { version: string } }).package.version);
    const installedCompiler = join(root, "node_modules/.bin/rust-js");
    expect(JSON.parse(run([runtime, installedCompiler, "--version-json"]))).toEqual(JSON.parse(run([compiler, "--version-json"])));
    // The app's Rust, a Cargo package (ADR 0101), its crate React's, named
    // by version: the plugin's patch tells Cargo it's in node_modules.
    mkdirSync(join(root, "src"));
    writeFileSync(join(root, "Cargo.toml"), `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[lib]\npath = "src/App.rs"\n\n[dependencies]\nreact = { package = "rust-js-react", version = "~${crateVersion("react")}" }\n\n[workspace]\n`);
    writeFileSync(join(root, "src", "App.rs"), `
#![allow(non_snake_case)]
use react::{JSX, jsx};
pub fn App() -> JSX::Element { jsx! { <main><span>{"Packaged"}</span></main> } }
`);
    writeFileSync(join(root, "check.js"), `
import rustJs from "@rust-js/vite-plugin";
import { publishArtifacts } from "@rust-js/build/publish";
import { createNativeBuilder } from "@rust-js/build/build";
import { planCargoLibraries } from "@rust-js/build/cargo";
const compiler = ${JSON.stringify(installedCompiler)};
const inputs = createNativeBuilder({ root: ${JSON.stringify(root)} }).watchFiles;
const symlinkInputs = createNativeBuilder({ root: ${JSON.stringify(root)}, rustJs: compiler }).watchFiles;
const plugin = rustJs({ cargo: { package: "app", offline: true } });
plugin.configResolved({ root: ${JSON.stringify(root)}, logger: { warn() {} } });
const watched = [];
await plugin.buildStart.call({ addWatchFile: file => watched.push(file), warn: message => { throw new Error(message); }, error: message => { throw new Error(message); } });
console.log(JSON.stringify({ watched, inputs, symlinkInputs, publisher: typeof publishArtifacts, cargoPlanner: typeof planCargoLibraries }));
`);
    const result = JSON.parse(run([runtime, "check.js"]));
    expect(result.watched).toContain(join(root, "src", "App.rs"));
    expect(result.publisher).toBe("function");
    expect(result.cargoPlanner).toBe("function");
    expect(result.inputs).toContain(join(root, "node_modules/@rust-js/native/bin/compiler"));
    expect(result.symlinkInputs).toContain(join(root, "node_modules/@rust-js/native/bin/compiler"));
    const jsx = readFileSync(join(root, "src", "App.jsx"), "utf8");
    expect(jsx).toContain("<main>");
    expect(jsx).toContain("<span>Packaged</span>");
    writeFileSync(join(bundle, "runtime.tgz"), "damaged archive");
    expect(() => run(["shasum", "-a", "256", "-c", "SHA256SUMS"], bundle)).toThrow();
    writeFileSync(join(root, "src", "App.rs"), "pub fn broken(");
    expect(() => run([runtime, "check.js"])).toThrow("unclosed delimiter");
    expect(readFileSync(join(root, "src", "App.jsx"), "utf8")).toBe(jsx);
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 600_000);
}
