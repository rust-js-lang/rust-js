// The same crates as a Cargo workspace (ADR 0101), built by Cargo with
// rust-js as its workspace wrapper: each library of the workspace compiled by
// rust-js, dependencies first, and serde, a crate of crates.io, by rustc, as
// Cargo would. `check`, a native binary calling `frontend::main`, is the oracle.

import { beforeAll, expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, rmSync, statSync, symlinkSync, utimesSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { node } from "./programs";
import { buildSerde } from "./serde-crate";
import { buildCompiler, cargoArtifacts, compiler, fixture, outsideFixture, root, run, rustJsUnits } from "./support";
import { cargo, check, printed, throughRustJs } from "./crates";
import { buildIdentity } from "../tooling/cargo.js";
import { compatNamed } from "../tooling/patch.js";

beforeAll(buildCompiler, 600_000);

// serde and serde_json, as the metadata rust-js reads.
const serde = { rmeta: () => buildSerde("rmeta") };

const members: Record<string, string> = {
  validation: "",
  models: 'validation = { path = "../validation" }\nserde = { version = "1", features = ["derive"] }\n',
  frontend: 'models = { path = "../models" }\nvalidation = { path = "../validation" }\nserde_json = "1"\n',
};

/** The workspace, with the lockfile serde/build.sh's serde was built from. */
function workspace(dir: string): string {
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["validation", "models", "frontend", "shell", "check"]\nresolver = "2"\n');
  cpSync(join(root, "serde", "Cargo.lock"), join(dir, "Cargo.lock"));
  for (const [name, uses] of Object.entries(members)) {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n${uses}`);
    cpSync(join(root, "test", "crates", name, "lib.rs"), join(dir, name, "src", "lib.rs"));
  }
  // Using `frontend` only, and through it the rest.
  mkdirSync(join(dir, "shell", "src"), { recursive: true });
  writeFileSync(join(dir, "shell", "Cargo.toml"), '[package]\nname = "shell"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nfrontend = { path = "../frontend" }\n');
  writeFileSync(join(dir, "shell", "src", "lib.rs"), "pub fn main() {\n    frontend::main();\n}\n");
  mkdirSync(join(dir, "check", "src"), { recursive: true });
  writeFileSync(join(dir, "check", "Cargo.toml"), '[package]\nname = "check"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nfrontend = { path = "../frontend" }\n');
  writeFileSync(join(dir, "check", "src", "main.rs"), "fn main() {\n    frontend::main();\n}\n");
  return join(dir, "Cargo.toml");
}


test("a Cargo workspace built with rust-js as its wrapper prints what native Rust does, and follows an edit", async () => {
  serde.rmeta();
  const dir = fixture("crates-cargo");
  const manifest = workspace(dir);
  const native = () => run(cargo(manifest, "run", "-p", "check"));
  // `models` built first on its own, the package Cargo was asked for, then used.
  await check(manifest, { packageNames: ["models"] });
  const { js, crates } = await check(manifest);
  expect(crates.map((c) => c.crate).sort()).toEqual(["frontend", "models", "validation"]);
  // serde's crates, which rust-js reads as rustc builds them (ADR 0077),
  // aren't tried, as another crate of crates.io's is (ADR 0415): none says
  // why it isn't compiled.
  const profile = join(dir, "target", "wasm32-unknown-unknown", "debug");
  expect(cargoArtifacts(profile).map((file) => basename(file)).filter((file) => file.startsWith("libserde") && file.endsWith(".rust-js-refused"))).toEqual([]);
  const before = printed(js!);
  expect(before).toBe(native());
  const validation = join(dir, "validation", "src", "lib.rs");
  writeFileSync(validation, readFileSync(validation, "utf8").replace("isn't an email address", "is no email address"));
  const after = printed((await check(manifest)).js!);
  expect(after).toBe(native());
  expect(after).not.toBe(before);
  // A crate told of the libraries its dependency was compiled against.
  expect(printed((await check(manifest, { packageNames: ["shell"] })).js!)).toBe(after);
}, 600_000);

// Which rust-js runs is in each crate for the target's dep-info, which Cargo
// reads of it as of an `env!`: `RUST_JS_BUILD`, which the tooling asks of
// the compiler. A build by another rust-js, an app's upgrade or its
// rollback, is another build, as an installed compiler's file is as old as
// its package says: by its date alone, Cargo had the other one's crates as
// done. Found upgrading the pilot to 0.0.3 and back. And which build of it,
// a hash of what it's made of: another checkout's or worktree's compiler of
// one version had a workspace's crates as done, and their libraries' facts
// were another build's (ADR 1003). It was what `rustc -vV` said through the
// wrapper, which Cargo hashes into every crate's `-C metadata`, the host's
// too: every build of rust-js built its build scripts and procedural macros
// again, and gave each crate another `stable_crate_id` (#2473). A crate for
// the target is built only where Cargo was given this compiler's own.
test("Cargo is told which rust-js it runs, so another one's crates are built again", async () => {
  serde.rmeta();
  const identity = JSON.parse(run([compiler, "--version-json"]));
  expect(identity.build).toMatch(/^[0-9a-f]{16}$/);
  const line = `rust-js: ${identity.version}, Rust ${identity.toolchain}, ABI ${identity.abi}, build ${identity.build}`;
  // With no printer of declarations named, whose hash it says too.
  expect(run([compiler, "--build-identity"], undefined, { RUST_JS_TYPESCRIPT: undefined })).toBe(`${line}\n`);
  // `-vV` names the wrapper, never which rust-js it is.
  const wrapped = "rust-js: the wrapper, its build in each target crate's dep-info";
  expect(run([compiler, "rustc", "-vV"])).toBe(`${run(["rustc", "-vV"])}${wrapped}\n`);
  // As Cargo asked it and kept what it said, and as each crate's dep-info names it.
  const dir = fixture("crates-cargo-identity");
  await check(workspace(dir));
  const asked = readFileSync(join(dir, "target", ".rustc_info.json"), "utf8");
  expect([asked.includes(wrapped), asked.includes(identity.build)]).toEqual([true, false]);
  const profile = join(dir, "target", "wasm32-unknown-unknown", "debug");
  const recorded = cargoArtifacts(profile).filter((f) => basename(f).startsWith("validation-") && f.endsWith(".d"));
  expect(recorded.map((f) => readFileSync(f, "utf8").includes(`# env-dep:RUST_JS_BUILD=${line}`))).toEqual([true]);
  // A crate for the target that Cargo gave no build, or another's.
  const source = join(dir, "lone.rs");
  writeFileSync(source, "pub fn answer() -> u32 {\n    42\n}\n");
  const lone = ["rustc", "--crate-name", "lone", "--crate-type", "lib", "--target", "wasm32-unknown-unknown", "--edition=2024", source, "--out-dir", join(dir, "lone")];
  for (const given of [undefined, `${line}, another`]) {
    const { RUST_JS_BUILD: _, ...rest } = process.env;
    // As the tooling gives it, past which the wrapper asks the build.
    const env = { ...rest, RUST_JS_COMPAT: compatNamed() };
    const built = Bun.spawnSync([compiler, ...lone], { cwd: dir, env: given === undefined ? env : { ...env, RUST_JS_BUILD: given } });
    expect([built.exitCode, built.stderr.toString()]).toEqual([1, expect.stringContaining("not this compiler's own build")]);
  }
}, 600_000);

// And which `@rust-js/typescript` prints its `.d.ts`, which isn't in the
// binary: a checkout's `typescript/index.js` changed, at the same path, and
// a regen of React's port kept the `.d.ts` the printer before it had printed,
// by Cargo's fingerprints, until its cache was cleared by hand (#675). Of
// what it is, not its date: one copied in as old is a build again too.
test("Cargo is told which declarations printer rust-js runs, so another one's crates are built again", async () => {
  const dir = fixture("cargo-printer");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["app"]\nresolver = "2"\n');
  mkdirSync(join(dir, "app", "src"), { recursive: true });
  writeFileSync(join(dir, "app", "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "app", "src", "lib.rs"), "pub fn answer(n: u32) -> u32 {\n    n + 1\n}\n");
  // A printer of its own, as a checkout's is, beside the TypeScript it runs.
  const printer = join(dir, "printer");
  mkdirSync(printer);
  for (const file of ["index.js", "package.json"]) cpSync(join(root, "typescript", file), join(printer, file));
  symlinkSync(join(root, "typescript", "node_modules"), join(printer, "node_modules"));
  const module = join(printer, "index.js");
  const manifest = join(dir, "Cargo.toml");
  // The `.d.ts` of the build of `app` Cargo has now, built or found done,
  // in the environment each build gives Cargo, the printer's identity in
  // its `RUST_JS_BUILD`, as the tooling asks it of its own printer.
  const declared = async () => {
    const env: NodeJS.ProcessEnv = { ...(await throughRustJs(manifest)), RUST_JS_TYPESCRIPT: module };
    env.RUST_JS_BUILD = buildIdentity(compiler, env);
    const messages = run(cargo(manifest, "check", "--target", "wasm32-unknown-unknown", "--message-format=json"), 600_000, env);
    const built = messages.split("\n").filter(Boolean).map((line) => JSON.parse(line))
      .find((message) => message.reason === "compiler-artifact" && message.target.name === "app");
    const metadata: string = built.filenames.find((file: string) => file.endsWith(".rmeta"));
    // Its JS's manifest, as the marker beside its metadata names it, which tooling reads.
    const library = readFileSync(metadata.replace(/\.rmeta$/, ".rust-js"), "utf8").split("\n")[0];
    return readFileSync(join(dirname(library), "lib.d.ts"), "utf8");
  };
  const before = await declared();
  expect(before).toContain("export function answer(n: number): number;");
  const { mtime } = statSync(module);
  writeFileSync(module, readFileSync(module, "utf8").replace("texts.push(await ts.print(module));", 'texts.push((await ts.print(module)) + "// another printer\'s\\n");'));
  utimesSync(module, mtime, mtime);
  expect(await declared()).toBe(`${before}// another printer's\n`);
}, 600_000);

// Another rust-js, here another declarations printer, builds each crate for
// the target again, the app's and a binding crate rustc checks, but under
// the same names: its `-C metadata`, of which its file names and its
// `stable_crate_id` are made, a brand's key (#2403), is Cargo's of rustc's
// own `-vV`. And a build script's and a procedural macro's, for the host,
// which no rust-js decision reaches, are done: it named rust-js in `-vV`,
// and every build of the compiler built them again (#2473).
test("another rust-js builds each crate for the target again, under the same names, and none for the host", async () => {
  const dir = fixture("cargo-identity-host");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["app", "maker", "dom"]\nresolver = "2"\n');
  const crate = (name: string, manifest: string, files: Record<string, string>) => {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n${manifest}`);
    for (const [file, text] of Object.entries(files)) writeFileSync(join(dir, name, file), text);
  };
  crate("maker", "\n[lib]\nproc-macro = true\n", { "src/lib.rs": 'use proc_macro::TokenStream;\n\n#[proc_macro]\npub fn answer(_: TokenStream) -> TokenStream {\n    "41".parse().unwrap()\n}\n' });
  crate("dom", "\n[package.metadata.rust-js]\nbindings = true\n", {
    "src/lib.rs": '#[cfg_attr(rust_js, rust_js::link_name = "@floating-ui/dom#computePosition")]\npub fn compute_position(x: f64) -> f64 {\n    unreachable!("{x}")\n}\n',
  });
  crate("app", '\n[dependencies]\nmaker = { path = "../maker" }\ndom = { path = "../dom" }\n', {
    "build.rs": 'fn main() {\n    println!("cargo::rustc-env=APP_GREETING=hello");\n}\n',
    "src/lib.rs": 'pub fn answer() -> u32 {\n    maker::answer!() + 1\n}\n\npub fn greeting() -> &\'static str {\n    env!("APP_GREETING")\n}\n\npub fn position() -> f64 {\n    dom::compute_position(1.0)\n}\n',
  });
  const printer = join(dir, "printer");
  mkdirSync(printer);
  for (const file of ["index.js", "package.json"]) cpSync(join(root, "typescript", file), join(printer, file));
  symlinkSync(join(root, "typescript", "node_modules"), join(printer, "node_modules"));
  const module = join(printer, "index.js");
  const manifest = join(dir, "Cargo.toml");
  // Each unit Cargo built or found done: its package, its kind, whether it
  // was done, and its files.
  const units = async () => {
    const env: NodeJS.ProcessEnv = { ...(await throughRustJs(manifest)), RUST_JS_TYPESCRIPT: module };
    env.RUST_JS_BUILD = buildIdentity(compiler, env);
    const messages = run(cargo(manifest, "check", "--target", "wasm32-unknown-unknown", "--message-format=json"), 600_000, env);
    // A procedural macro of the workspace's twice: checked, and built for its user.
    return messages.split("\n").filter(Boolean).map((line) => JSON.parse(line))
      .filter((message) => message.reason === "compiler-artifact")
      .map((message) => ({ unit: `${message.target.name} ${message.target.kind.join(",")}`, fresh: message.fresh as boolean, files: (message.filenames as string[]).map((file) => basename(file)) }))
      .sort((a, b) => `${a.unit} ${a.files}`.localeCompare(`${b.unit} ${b.files}`));
  };
  const first = await units();
  expect([...new Set(first.map(({ unit }) => unit))]).toEqual(["app lib", "build-script-build custom-build", "dom lib", "maker proc-macro"]);
  // Each crate for the target names the build in its dep-info.
  const profile = join(dir, "target", "wasm32-unknown-unknown", "debug");
  for (const name of ["app", "dom"]) {
    const recorded = cargoArtifacts(profile).filter((f) => basename(f).startsWith(`${name}-`) && f.endsWith(".d"));
    expect(recorded.map((f) => readFileSync(f, "utf8").includes("# env-dep:RUST_JS_BUILD=rust-js: "))).toEqual([true]);
  }
  // What the last build wrote is another build's, of its own schema, here
  // a library contract this compiler's lacks each field of: the build over
  // it reads only what it wrote, which it may remove. A kept app of the
  // guide's failed, "missing field `crate`", reading the whole of the one
  // the last compiler wrote (2026-11-25).
  const written = readdirSync(join(profile, "rust-js")).map((unit) => join(profile, "rust-js", unit, "lib.manifest.json")).filter((file) => existsSync(file));
  expect(written.length).toBeGreaterThan(0);
  for (const file of written) {
    const was = JSON.parse(readFileSync(file, "utf8"));
    writeFileSync(file, JSON.stringify({ ...was, compiler: { ...was.compiler, build: "another" }, library: { another: true } }));
  }
  writeFileSync(module, `${readFileSync(module, "utf8")}\n// another printer's\n`);
  const second = await units();
  expect(second.map(({ unit, fresh, files }) => ({ unit, fresh, files }))).toEqual(
    first.map(({ unit, files }) => ({ unit, fresh: unit.endsWith(" proc-macro") || unit.endsWith(" custom-build"), files })),
  );
}, 600_000);

// A brand's key is its crate's identity (ADR 1196, amended): its name and
// a hash of its package's name, version and source, which tells two
// versions of one crate apart, a git fork's and a registry's too, and two
// crates of one package, renamed or not; never where its files are, nor
// the compiler's build, which a rebuild of rust-js changes, or another
// printer, as here. rustc's `StableCrateId` is of Cargo's `-C metadata`,
// which hashes a path dependency outside the workspace by its directory,
// so React's port, whose bindings are `../rust-js`, branded each type by
// another key in each checkout it was built in, and its pinned JS
// differed in every other (#2403). #2473 keeps `rustc -vV` one line for
// every build, rust-js's identity in its crates' dep-info alone.
test("a brand's key is the same of every build of rust-js and every directory, and two versions' are two", async () => {
  // The app the workspace's, the two versions path dependencies of it
  // outside the workspace, as Cargo takes no two members of one name: the
  // same files in two directories.
  const sealed = "pub struct Email(String);\n\npub fn make(text: &str) -> Email {\n    Email(text.to_string())\n}\n\npub fn domain(email: &Email) -> usize {\n    email.0.len()\n}\n";
  const laid = (dir: string) => {
    mkdirSync(join(dir, "ws", "app", "src"), { recursive: true });
    writeFileSync(join(dir, "ws", "Cargo.toml"), '[workspace]\nmembers = ["app"]\nresolver = "2"\n');
    for (const [member, version] of [["one", "0.1.0"], ["two", "0.2.0"]]) {
      mkdirSync(join(dir, member, "src"), { recursive: true });
      writeFileSync(join(dir, member, "Cargo.toml"), `[package]\nname = "sealed"\nversion = "${version}"\nedition = "2024"\n`);
      writeFileSync(join(dir, member, "src", "lib.rs"), sealed);
    }
    writeFileSync(join(dir, "ws", "app", "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\none = { package = "sealed", path = "../../one" }\ntwo = { package = "sealed", path = "../../two" }\n');
    writeFileSync(join(dir, "ws", "app", "src", "lib.rs"), "pub fn both(a: &str, b: &str) -> usize {\n    one::domain(&one::make(a)) + two::domain(&two::make(b))\n}\n");
    const printer = join(dir, "printer");
    mkdirSync(printer);
    for (const file of ["index.js", "package.json"]) cpSync(join(root, "typescript", file), join(printer, file));
    symlinkSync(join(root, "typescript", "node_modules"), join(printer, "node_modules"));
    return { manifest: join(dir, "ws", "Cargo.toml"), module: join(printer, "index.js") };
  };
  // Each crate's JS, by its package's name and version, of the build Cargo
  // has now.
  const built = async ({ manifest, module }: { manifest: string; module: string }) => {
    const env: NodeJS.ProcessEnv = { ...(await throughRustJs(manifest)), RUST_JS_TYPESCRIPT: module };
    env.RUST_JS_BUILD = buildIdentity(compiler, env);
    const messages = run(cargo(manifest, "check", "--target", "wasm32-unknown-unknown", "--message-format=json"), 600_000, env);
    const artifacts = messages.split("\n").filter(Boolean).map((line) => JSON.parse(line))
      .filter((message) => message.reason === "compiler-artifact" && ["sealed", "app"].includes(message.target.name));
    return Object.fromEntries(artifacts.map((message) => {
      const metadata: string = message.filenames.find((file: string) => file.endsWith(".rmeta"));
      const library = readFileSync(metadata.replace(/\.rmeta$/, ".rust-js"), "utf8").split("\n")[0];
      return [`${message.target.name}@${String(message.package_id).split(/[#@]/).pop()}`, readFileSync(join(dirname(library), "lib.js"), "utf8")];
    }));
  };
  const key = (js: string) => /\$brand\("([^"]+)"/.exec(js)?.[1];
  const here = laid(fixture("cargo-brand-keys"));
  const before = await built(here);
  expect(Object.keys(before).sort()).toEqual(["app@0.1.0", "sealed@0.1.0", "sealed@0.2.0"]);
  expect(key(before["sealed@0.1.0"])).toStartWith("sealed#");
  expect(key(before["sealed@0.1.0"])).not.toBe(key(before["sealed@0.2.0"]));
  // Another build of rust-js, as Cargo sees it: another printer.
  writeFileSync(here.module, `${readFileSync(here.module, "utf8")}// another build's\n`);
  expect(await built(here)).toEqual(before);
  // The same files in another directory: each library's JS the same, byte
  // for byte, its brands' keys too. Not yet the app's, whose imports name
  // Cargo's directory of each library, `sealed-<its -C metadata>`, and are
  // named in that order (#2616).
  const elsewhere = await built(laid(fixture("cargo-brand-keys-elsewhere")));
  for (const version of ["sealed@0.1.0", "sealed@0.2.0"]) expect(elsewhere[version]).toBe(before[version]);
}, 600_000);

// What Cargo has as done is what rust-js made (ADR 0101): Cargo rebuilds a
// crate when what its record of the sources lists changes, which rust-js adds
// itself to, and a crate's JS is beside its metadata.
test("a Cargo build of rust-js's crates is done when their JS is, and not before", async () => {
  serde.rmeta();
  const dir = fixture("crates-cargo-fresh");
  const manifest = workspace(dir);
  const { js, crates } = await check(manifest);
  const written = statSync(js!).mtimeMs;
  expect((await check(manifest)).js).toBe(js);
  expect(statSync(js!).mtimeMs).toBe(written);
  const profile = join(dir, "target", "wasm32-unknown-unknown", "debug");
  expect(rustJsUnits(profile)).toContain(dirname(js!));
  const recorded = cargoArtifacts(profile).filter((f) => basename(f).startsWith("validation-") && f.endsWith(".d")).map((f) => readFileSync(f, "utf8"));
  expect(recorded.length).toBe(1);
  // The compiler that ran, which a change to makes Cargo build again: an
  // installed launcher's is the binary beside it, which it runs.
  const launcher = readFileSync(compiler).subarray(0, 2).toString() === "#!";
  const binary = launcher ? join(dirname(realpathSync(compiler)), "compiler") : compiler;
  // A dep-info file's paths, its spaces escaped, `\ `, past the output's;
  // and the `Cargo.toml` its settings are of, a change of which alone makes
  // Cargo build it again too (#1751).
  const named = recorded[0].split("\n")[0].split(/(?<!\\) /).slice(1).map((path) => path.replaceAll("\\ ", " "));
  expect(named.filter((path) => existsSync(path)).map((path) => realpathSync(path))).toEqual(expect.arrayContaining([realpathSync(binary), realpathSync(join(dir, "validation", "Cargo.toml"))]));
  // A library's JS gone from a build Cargo has as done: refused, not given to
  // an app that imports it, until Cargo is told to build it again. Found in
  // review.
  rmSync(crates.find((c) => c.crate === "models")!.js!);
  await expect(check(manifest)).rejects.toThrow("cargo clean -p models --target wasm32-unknown-unknown");
  run(cargo(manifest, "clean", "-p", "models", "--target", "wasm32-unknown-unknown"));
  expect(printed((await check(manifest)).js!)).toBe(printed(js!));
  // A library's manifest gone, the crates using it are refused, not compiled
  // as if it were a crate rustc built.
  rmSync(dirname(crates.find((c) => c.crate === "validation")!.manifest), { recursive: true });
  const models = join(dir, "models", "src", "lib.rs");
  writeFileSync(models, readFileSync(models, "utf8") + "\npub fn more() {}\n");
  await expect(check(manifest)).rejects.toThrow("`cargo clean` to build it again");
  const env = await throughRustJs(manifest);
  expect(() => run(cargo(manifest, "build", "--target", "wasm32-unknown-unknown", "-p", "validation"), 600_000, env))
    .toThrow("`cargo build` asks for what rustc links");
}, 600_000);

// The app's Prettier config lays its JS out (ADR 0381), so Cargo is told of
// it, as of the sources: a change to it is a build again.
test("a Cargo build is done until the app's Prettier config changes", async () => {
  const dir = fixture("cargo-prettier");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["app"]\nresolver = "2"\n');
  mkdirSync(join(dir, "app", "src"), { recursive: true });
  writeFileSync(join(dir, "app", "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "app", "src", "lib.rs"), 'pub fn greeting() -> &\'static str {\n    "hi"\n}\n');
  writeFileSync(join(dir, ".prettierrc"), "{}\n");
  const manifest = join(dir, "Cargo.toml");
  const built = async () => readFileSync((await check(manifest, { packageNames: ["app"] })).js!, "utf8");
  expect(await built()).toContain('return "hi";');
  writeFileSync(join(dir, ".prettierrc"), '{"singleQuote": true}\n');
  expect(await built()).toContain("return 'hi';");
}, 600_000);

// And which Prettier: an upgrade of the app's lays its JS out otherwise, of
// a package Cargo dates nothing of, its configs the same, and every crate
// was fresh, its old layout kept (#684). Of what's installed, not its date:
// one copied in as old as the cache is another build too.
test("Cargo is told which Prettier lays out the app's JS, so another one's crates are built again", async () => {
  const dir = fixture("cargo-prettier-upgrade");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["app"]\nresolver = "2"\n');
  mkdirSync(join(dir, "app", "src"), { recursive: true });
  writeFileSync(join(dir, "app", "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "app", "src", "lib.rs"), 'pub fn greeting() -> &\'static str {\n    "hi"\n}\n');
  writeFileSync(join(dir, ".prettierrc"), "{}\n");
  // The app's Prettier, at a release that says it's laid the file out.
  const prettier = join(dir, "node_modules", "prettier");
  mkdirSync(prettier, { recursive: true });
  const install = (version: string) => {
    writeFileSync(join(prettier, "package.json"), JSON.stringify({ name: "prettier", version, main: "index.js" }));
    writeFileSync(join(prettier, "index.js"), `module.exports = { resolveConfigFile: async () => ${JSON.stringify(join(dir, ".prettierrc"))}, `
      + `resolveConfig: async () => ({}), format: async (code) => "// laid out by ${version}\\n" + code };\n`);
  };
  install("3.0.0");
  const manifest = join(dir, "Cargo.toml");
  const built = async () => readFileSync((await check(manifest, { packageNames: ["app"] })).js!, "utf8");
  expect(await built()).toStartWith("// laid out by 3.0.0\n");
  // Upgraded at the same path, each file dated as it was.
  const dates = ["package.json", "index.js"].map((file) => [join(prettier, file), statSync(join(prettier, file)).mtime] as const);
  install("3.0.1");
  for (const [file, mtime] of dates) utimesSync(file, mtime, mtime);
  expect(await built()).toStartWith("// laid out by 3.0.1\n");
}, 600_000);

// And which plugins its config names: react-in-rust's
// `prettier-plugin-hermes-parser` upgraded alone, Prettier and its config
// the same, kept every crate's old layout (#692). Each found where Prettier
// loads it from, where the crate is.
test("Cargo is told which Prettier plugins lay out the app's JS, so another one's crates are built again", async () => {
  const dir = fixture("cargo-prettier-plugin-upgrade");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["app"]\nresolver = "2"\n');
  mkdirSync(join(dir, "app", "src"), { recursive: true });
  writeFileSync(join(dir, "app", "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "app", "src", "lib.rs"), 'pub fn greeting() -> &\'static str {\n    "hi"\n}\n');
  writeFileSync(join(dir, ".prettierrc"), '{"plugins": ["prettier-plugin-stamp"]}\n');
  // The app's Prettier, which lays a file out by each plugin its config
  // names, loaded where it runs, as Prettier's `loadPlugin` does.
  const prettier = join(dir, "node_modules", "prettier");
  mkdirSync(prettier, { recursive: true });
  writeFileSync(join(prettier, "package.json"), JSON.stringify({ name: "prettier", version: "3.0.0", main: "index.js" }));
  writeFileSync(join(prettier, "index.js"), `const { createRequire } = require("node:module");\nconst { join } = require("node:path");\n`
    + `module.exports = { resolveConfigFile: async () => ${JSON.stringify(join(dir, ".prettierrc"))}, `
    + `resolveConfig: async () => ({ plugins: ["prettier-plugin-stamp"] }), `
    + `format: async (code, { plugins = [] }) => plugins.reduce((text, name) => createRequire(join(process.cwd(), "noop.js"))(name).stamp(text), code) };\n`);
  const plugin = join(dir, "node_modules", "prettier-plugin-stamp");
  mkdirSync(plugin, { recursive: true });
  const install = (version: string) => {
    writeFileSync(join(plugin, "package.json"), JSON.stringify({ name: "prettier-plugin-stamp", version, main: "index.js" }));
    writeFileSync(join(plugin, "index.js"), `module.exports = { stamp: (code) => "// stamped by ${version}\\n" + code };\n`);
  };
  install("1.0.0");
  const manifest = join(dir, "Cargo.toml");
  const built = async () => readFileSync((await check(manifest, { packageNames: ["app"] })).js!, "utf8");
  expect(await built()).toStartWith("// stamped by 1.0.0\n");
  // The plugin upgraded alone, at the same path, each file dated as it was.
  const dates = ["package.json", "index.js"].map((file) => [join(plugin, file), statSync(join(plugin, file)).mtime] as const);
  install("1.0.1");
  for (const [file, mtime] of dates) utimesSync(file, mtime, mtime);
  expect(await built()).toStartWith("// stamped by 1.0.1\n");
}, 600_000);

/** A workspace of one library, `app`, whose `value` its feature `alternate` changes. */
function featured(dir: string): string {
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["app"]\nresolver = "2"\n');
  mkdirSync(join(dir, "app", "src"), { recursive: true });
  writeFileSync(join(dir, "app", "Cargo.toml"), '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n\n[features]\nalternate = []\n');
  writeFileSync(join(dir, "app", "src", "lib.rs"),
    '#[cfg(not(feature = "alternate"))]\npub fn value() -> u32 {\n    1\n}\n#[cfg(feature = "alternate")]\npub fn value() -> u32 {\n    2\n}\n');
  return join(dir, "Cargo.toml");
}
const value = (js: string) => run([node ?? "node", "--input-type=module", "--eval", `console.log((await import(${JSON.stringify(js)})).value());`]).trim();

// Found in review: each feature set's build wrote one place, and Cargo, with
// the first's cached, had the second's JS as the first's.
test("a Cargo build of another feature set, and back, is each one's JS", async () => {
  const manifest = featured(fixture("cargo-features"));
  const values = [];
  for (const features of [[], ["alternate"], []]) values.push(value((await check(manifest, { packageNames: ["app"], features })).js!));
  expect(values).toEqual(["1", "2", "1"]);
}, 600_000);

// Found in review: a crate rustc checked, fresh when rust-js was asked for.
test("a crate Cargo checked without rust-js is compiled by it once it's the wrapper", async () => {
  const manifest = featured(fixture("cargo-wrapper-added"));
  run(cargo(manifest, "check", "--target", "wasm32-unknown-unknown"));
  expect(value((await check(manifest, { packageNames: ["app"] })).js!)).toBe("1");
}, 600_000);

// Found in review: what Cargo is told of a crate was written after its JS was
// published, so a build that failed writing it had changed the JS.
test("a Cargo build that can't record what it made leaves the previous build's JS", async () => {
  const dir = fixture("cargo-record-blocked");
  const manifest = featured(dir);
  const { js } = await check(manifest, { packageNames: ["app"] });
  const before = readFileSync(js!, "utf8");
  const profile = join(dir, "target", "wasm32-unknown-unknown", "debug");
  const marker = cargoArtifacts(profile).find((f) => f.endsWith(".rust-js"))!;
  rmSync(marker);
  mkdirSync(join(marker, "in the way"), { recursive: true });
  const lib = join(dir, "app", "src", "lib.rs");
  writeFileSync(lib, readFileSync(lib, "utf8").replace("    1\n", "    3\n"));
  await expect(check(manifest, { packageNames: ["app"] })).rejects.toThrow();
  expect(readFileSync(js!, "utf8")).toBe(before);
}, 600_000);
// Another crate's derive, as strum's and thiserror's are: what it writes is
// `#[automatically_derived]`, as std's derives' is, but it's the crate's own
// code, called, given as a dictionary, and a `From` that `?` uses.
test("what another crate's derive writes is compiled as the crate's own code", async () => {
  const dir = fixture("cargo-derive");
  const crates: Record<string, [string, string, string]> = {
    describe_derive: ["", "[lib]\nproc-macro = true\n", "use proc_macro::TokenStream;\n\n// The name after `struct` or `enum`: enough for the types it's given.\nfn name(input: &TokenStream) -> String {\n    let text = input.to_string();\n    let mut words = text.split_whitespace();\n    while let Some(word) = words.next() {\n        if word == \"struct\" || word == \"enum\" {\n            let next = words.next().unwrap();\n            return next.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();\n        }\n    }\n    panic!(\"no type\")\n}\n\n#[proc_macro_derive(Describe)]\npub fn describe(input: TokenStream) -> TokenStream {\n    let name = name(&input);\n    format!(\"#[automatically_derived] impl ::describe::Describe for {name} {{ fn describe(&self) -> String {{ format!(\\\"{name}: {{:?}}\\\", self) }} }}\")\n        .parse()\n        .unwrap()\n}\n\n#[proc_macro_derive(FromCode)]\npub fn from_code(input: TokenStream) -> TokenStream {\n    let name = name(&input);\n    format!(\"#[automatically_derived] impl ::core::convert::From<u32> for {name} {{ fn from(code: u32) -> Self {{ {name}::Code(code) }} }}\")\n        .parse()\n        .unwrap()\n}\n"],
    describe: ['describe_derive = { path = "../describe_derive" }\n', "", "pub use describe_derive::{Describe, FromCode};\n\npub trait Describe {\n    fn describe(&self) -> String;\n    fn shout(&self) -> String {\n        self.describe().to_uppercase()\n    }\n}\n"],
    app: ['describe = { path = "../describe" }\n', "", "use describe::{Describe, FromCode};\n\n#[derive(Debug, Describe)]\npub struct Point(pub i32, pub i32);\n\n#[derive(Debug, Describe, FromCode)]\npub enum Failure {\n    Code(u32),\n    Other,\n}\n\nfn all<T: Describe>(items: &[T]) -> Vec<String> {\n    items.iter().map(|item| item.describe()).collect()\n}\n\nfn verify(code: u32) -> Result<(), Failure> {\n    let found: Result<(), u32> = if code > 0 { Err(code) } else { Ok(()) };\n    found?;\n    Ok(())\n}\n\npub fn main() {\n    println!(\"{}\", Point(1, 2).describe());\n    println!(\"{}\", Point(3, 4).shout());\n    println!(\"{:?}\", all(&[Failure::Code(7), Failure::Other]));\n    println!(\"{:?} {:?}\", verify(0), verify(5));\n    let failure: Failure = 9.into();\n    println!(\"{}\", failure.describe());\n}\n"],
  };
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["describe_derive", "describe", "app", "check"]\nresolver = "2"\n');
  for (const [name, [uses, lib, source]] of Object.entries(crates)) {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n\n${lib}[dependencies]\n${uses}`);
    writeFileSync(join(dir, name, "src", "lib.rs"), source);
  }
  mkdirSync(join(dir, "check", "src"), { recursive: true });
  writeFileSync(join(dir, "check", "Cargo.toml"), '[package]\nname = "check"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\napp = { path = "../app" }\n');
  writeFileSync(join(dir, "check", "src", "main.rs"), "fn main() {\n    app::main();\n}\n");
  const manifest = join(dir, "Cargo.toml");
  const { js } = await check(manifest, { packageNames: ["app"] });
  expect(printed(js!)).toBe(run(cargo(manifest, "run", "-p", "check")));
}, 600_000);

// The app, the package the tooling builds, is compiled as a program where
// no crate of the workspace uses it: an `Rc` whose counts nothing reads is
// the value it points at. One another crate of the workspace uses is a
// library whichever package is built, its every `Rc` counted where it reads
// a count, as the crates sharing it count it (ADRs 0523, 0844): a crate's JS is the same however
// it's reached, alone or through a dependent, its `dyn Fn` dropped as a
// library's is (ADR 0521), as React's `shared` is (ADR 0360, amended).
// Outside the checkout, where no Prettier is found: one found puts every
// crate's directory in Cargo's key (ADR 1003, #684), so a crate added to
// the workspace would rebuild each without its `RUST_JS_APPS` recorded.
test("the app's crate is a program where no crate uses it, and a library, the same bytes, where one does, whichever packages are built", async () => {
  const dir = outsideFixture("crates-app-program");
  const counts = "use std::rc::Rc;\n\npub struct Shared(pub Rc<u32>);\n\npub fn enqueue(task: Box<dyn Fn()>) {\n    task();\n}\n\npub fn main() {\n    let a = Rc::new(5);\n    let b = Rc::clone(&a);\n    println!(\"{} {} {}\", *a, *b, Rc::strong_count(&a));\n}\n";
  // `user` reads no count, and holds a `Copy` struct it never changes in
  // place, which neither a program nor a library copies (ADR 0914, amended).
  const reads = counts
    .replace(", Rc::strong_count(&a)", "")
    .replace("{} {} {}", "{} {}")
    .replace("pub fn main() {\n", "#[derive(Clone, Copy)]\npub struct Point {\n    pub x: u32,\n}\n\npub trait Shape {\n    fn area(&self) -> u32;\n}\n\npub struct Square(pub u32);\n\nimpl Shape for Square {\n    fn area(&self) -> u32 {\n        self.0 * self.0\n    }\n}\n\npub fn main() {\n    let p = Point { x: 1 };\n    let q = p;\n    println!(\"{}\", p.x + q.x);\n    let shape: Box<dyn Shape> = Box::new(Square(2));\n    println!(\"{}\", shape.area());\n");
  for (const [name, uses, source] of [
    ["app", "", counts],
    ["user", 'app = { path = "../app" }\n', reads.replace("println!", "app::main();\n    println!")],
  ]) {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n${uses}`);
    writeFileSync(join(dir, name, "src", "lib.rs"), source);
  }
  const manifest = join(dir, "Cargo.toml");
  const counted = (js: string) => readFileSync(js, "utf8").includes("strong: 1");
  writeFileSync(manifest, '[workspace]\nmembers = ["app", "user"]\nresolver = "2"\n');
  run(cargo(manifest, "generate-lockfile"));
  const profile = join(dir, "target", "wasm32-unknown-unknown", "debug");
  const checked = () => statSync(cargoArtifacts(profile).find((f) => basename(f).startsWith("libapp-") && f.endsWith(".rmeta"))!).mtimeMs;
  const app = await check(manifest, { packageNames: ["app"] });
  const alone = readFileSync(app.js!, "utf8");
  const first = checked();
  expect([printed(app.js!), counted(app.js!)]).toEqual(["5 5 2\n", true]);
  const user = await check(manifest, { packageNames: ["user"] });
  const copied = (js: string) => readFileSync(js, "utf8").includes("{ ...p }");
  // A program's own trait object, whose implementors drop nothing, needs
  // no drop; a library's may hold a consumer's that does (ADRs 0163, 0360).
  const disposed = (js: string) => readFileSync(js, "utf8").includes("shape.impl.$drop?.(shape.value);");
  expect([printed(user.js!), counted(user.js!), copied(user.js!), disposed(user.js!)]).toEqual(["5 5 2\n2\n4\n5 5\n", false, false, false]);
  expect(readFileSync(user.crates.find((c) => c.crate === "app")!.js!, "utf8")).toBe(alone);
  const own = readFileSync(user.js!, "utf8");
  const again = await check(manifest, { packageNames: ["app"] });
  expect(readFileSync(again.js!, "utf8")).toBe(alone);
  // Which programs there are is the workspace's, whichever package is
  // built: `app`, checked for itself, was done for `user`'s build and for
  // its own again, never checked twice (rd662).
  expect(checked()).toBe(first);
  // The whole workspace at once, each member a package Cargo was asked
  // for: each crate's JS is what its own build wrote, `user` a program,
  // which nothing uses, and `app` the library `user` uses (ADR 0360,
  // amended); there is no one package's JS.
  const whole = await check(manifest, { packageNames: [] });
  expect(whole.js).toBeUndefined();
  expect([readFileSync(whole.crates.find((c) => c.crate === "user")!.js!, "utf8"), readFileSync(whole.crates.find((c) => c.crate === "app")!.js!, "utf8")]).toEqual([own, alone]);
  expect(checked()).toBe(first);
  // Once a crate of the workspace uses `user`, it's a library, built again,
  // whose trait object may hold a consumer's that drops: which crates are
  // programs is recorded for Cargo (ADR 0360).
  mkdirSync(join(dir, "shell", "src"), { recursive: true });
  writeFileSync(join(dir, "shell", "Cargo.toml"), '[package]\nname = "shell"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nuser = { path = "../user" }\n');
  writeFileSync(join(dir, "shell", "src", "lib.rs"), "pub fn main() {\n    user::main();\n}\n");
  writeFileSync(manifest, '[workspace]\nmembers = ["app", "user", "shell"]\nresolver = "2"\n');
  run(cargo(manifest, "generate-lockfile"));
  const used = await check(manifest, { packageNames: ["shell"] });
  expect([printed(used.js!), disposed(used.crates.find((c) => c.crate === "user")!.js!)]).toEqual(["5 5 2\n2\n4\n5 5\n", true]);
  // Built alone, the package asked for, it's the same library.
  const library = readFileSync(used.crates.find((c) => c.crate === "user")!.js!, "utf8");
  expect(readFileSync((await check(manifest, { packageNames: ["user"] })).js!, "utf8")).toBe(library);
}, 600_000);

// `rust-js build` of a workspace's root builds every member at once, and
// `--package`, as often as Cargo's `-p`, those it names: each crate checked
// once, the independent ones on Cargo's cores, and each crate's JS, `.d.ts`
// and map in source byte for byte what a build of its own package writes
// (ADR 1068).
test("rust-js build builds a workspace's members at once, or the packages named, each crate's files what its own package's build writes", () => {
  const dir = fixture("crates-whole-workspace");
  writeFileSync(join(dir, "Cargo.toml"), '[workspace]\nmembers = ["app", "user", "other"]\nresolver = "2"\n');
  writeFileSync(join(dir, "package.json"), '{ "private": true, "type": "module" }\n');
  // `app`'s options, which `user` and `other` build and `app` reads, as
  // react-dom's `createRoot` reads its options: how it reads each field, a
  // place or a getter's, `null` or a value, is `app`'s own (#579).
  const options = "\npub struct Options {\n    pub prefix: Option<String>,\n    pub on_error: Option<fn() -> u32>,\n}\n\npub fn configure(options: Option<&Options>) -> String {\n    let mut prefix = String::new();\n    if let Some(options) = options {\n        if let Some(given) = &options.prefix {\n            prefix = given.clone();\n        }\n        if let Some(on_error) = options.on_error {\n            prefix.push_str(&on_error().to_string());\n        }\n    }\n    prefix\n}\n";
  const counts = "use std::rc::Rc;\n\npub fn count() -> usize {\n    let a = Rc::new(5);\n    let b = Rc::clone(&a);\n    *b + Rc::strong_count(&a)\n}\n" + options;
  const shapes = "pub trait Shape {\n    fn area(&self) -> u32;\n}\n\npub struct Square(pub u32);\n\nimpl Shape for Square {\n    fn area(&self) -> u32 {\n        self.0 * self.0\n    }\n}\n\nfn code() -> u32 {\n    7\n}\n\npub fn main() {\n    let shape: Box<dyn Shape> = Box::new(Square(2));\n    println!(\"{}\", shape.area() as usize + app::count());\n    let options = app::Options { prefix: Some(\"p\".to_string()), on_error: Some(code) };\n    println!(\"{}\", app::configure(Some(&options)));\n}\n";
  for (const [name, uses, source] of [["app", "", counts], ["user", 'app = { path = "../app" }\n', shapes], ["other", 'app = { path = "../app" }\n', shapes]]) {
    mkdirSync(join(dir, name, "src"), { recursive: true });
    writeFileSync(join(dir, name, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n${uses}`);
    writeFileSync(join(dir, name, "src", "lib.rs"), source);
  }
  run(cargo(join(dir, "Cargo.toml"), "generate-lockfile"));
  const build = (...args: string[]) => {
    const built = Bun.spawnSync([process.execPath, join(root, "tooling", "cli.js"), "build", ...args], { cwd: dir, env: { ...process.env, RUST_JS_COMPILER: compiler }, stdout: "pipe", stderr: "pipe" });
    expect([built.exitCode, built.stderr.toString()]).toEqual([0, ""]);
  };
  // Every file written beside the Rust, by its path, then gone, so the
  // next build writes each again.
  const written = () => {
    const files = new Map<string, string>();
    for (const name of ["app", "user", "other"]) {
      for (const file of readdirSync(join(dir, name, "src")).filter((file) => !file.endsWith(".rs")).sort()) {
        files.set(`${name}/src/${file}`, readFileSync(join(dir, name, "src", file), "utf8"));
        rmSync(join(dir, name, "src", file));
      }
    }
    return files;
  };
  build("--package", "user");
  build("--package", "other");
  const alone = written();
  expect([...alone.keys()]).toEqual(["app/src/lib.d.ts", "app/src/lib.js", "app/src/lib.js.map", "user/src/lib.d.ts", "user/src/lib.js", "user/src/lib.js.map", "other/src/lib.d.ts", "other/src/lib.js", "other/src/lib.js.map"]);
  // `user` and `other` are programs, which nothing uses: their own trait
  // object drops nothing; `app`, which both use, is a library (ADR 0360).
  expect(["user", "other", "app"].map((name) => alone.get(`${name}/src/lib.js`)!.includes("$drop?."))).toEqual([false, false, false]);
  expect(alone.get("app/src/lib.js")).toContain("strong: 1");
  // `app` built alone, with no crate that uses it, is the same library.
  build("--package", "app");
  const library = written();
  expect([...library.keys()]).toEqual(["app/src/lib.d.ts", "app/src/lib.js", "app/src/lib.js.map"]);
  expect([...library].map(([file, text]) => text === alone.get(file))).toEqual([true, true, true]);
  build();
  expect(written()).toEqual(alone);
  build("--package", "user", "--package", "other");
  expect(written()).toEqual(alone);
}, 600_000);
