// Native build preparation. Hosts provide scheduling and consume manifests.
import { spawn } from "node:child_process";
import { existsSync, readFileSync, realpathSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { cargoWorkspace, checkCargo, lintCheck, packageOf } from "./cargo.js";
import { parseCompilerIdentity } from "./manifest.js";

/** The rust-js checkout this `@rust-js/build` is of, where one runs it from
 * one; installed, it's of none. */
const thisCheckout = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** Where rust-js keeps what it makes for an app, outside what it commits:
 * its Cargo builds, and each host's own. */
export function cacheDirOf(root) {
  return join(root, "node_modules/.cache/rust-js");
}

/** What `rust-js build` says without a compiler at `rustJs`: the JS
 * committed beside the Rust is what an app builds from without one (ADR
 * 0041), but nothing compiles the Rust. */
export function withoutCompiler(rustJs) {
  return `no rust-js at ${rustJs}: install rust-js or build it with cargo build to compile the Rust`;
}

/**
 * The compiler a host runs for an app at `root`, the one answer every host
 * and command asks: `RUST_JS_COMPILER` where it names one, a test's or a
 * script's, as it is; else the app's `@rust-js/native`, a release's; else
 * the rust-js checkout's own, `checkout`'s, built from its sources first
 * (ADR 0854).
 * @param {string} root
 * @param {{ checkout?: string }} [options]
 * @returns {Promise<string>}
 */
export async function findCompiler(root, { checkout = thisCheckout } = {}) {
  if (process.env.RUST_JS_COMPILER) return resolve(root, process.env.RUST_JS_COMPILER);
  const require = createRequire(join(root, "package.json"));
  try { return join(dirname(require.resolve("@rust-js/native/package.json")), "bin/rust-js"); }
  catch (error) {
    if (error.code !== "MODULE_NOT_FOUND") throw error;
  }
  return checkoutCompiler(checkout);
}

/**
 * A rust-js checkout's own compiler, `target/debug/rust-js`, built from the
 * checkout's sources as they are now by its `build:compiler`, the one
 * definition, which `bun run build` runs too: Cargo's, which does nothing
 * when nothing changed, and waits for another host's build of it to finish
 * (ADR 0854). A build that fails throws: the binary an earlier one left is
 * of other sources, and isn't run. An installed `@rust-js/build` is of no
 * checkout, with nothing to build, and the path is of no compiler.
 * @param {string} checkout
 * @returns {Promise<string>}
 */
async function checkoutCompiler(checkout) {
  const compiler = join(checkout, "target/debug/rust-js");
  const manifest = join(checkout, "package.json");
  const workspace = existsSync(manifest) ? JSON.parse(readFileSync(manifest, "utf8")) : {};
  if (workspace.name !== "@rust-js/workspace") return compiler;
  // As a shell in the checkout runs it: with `.cargo/config.toml`'s
  // `RUSTC_BOOTSTRAP`, as another rebuilds every dependency, and into the
  // target directory the compiler is run from, whatever the app's is.
  const { RUSTC_BOOTSTRAP: _, ...env } = process.env;
  const code = await new Promise((resolve, reject) => {
    // What it says is on stderr, as Cargo prints it: an editor's check
    // reads Cargo's messages from stdout.
    const child = spawn("sh", ["-c", workspace.scripts["build:compiler"]], {
      cwd: checkout,
      env: { ...env, PWD: checkout, CARGO_TARGET_DIR: join(checkout, "target") },
      stdio: ["ignore", 2, 2],
    });
    child.on("error", reject);
    child.on("close", resolve);
  });
  if (code !== 0) {
    throw new Error(`rust-js: building ${compiler} from this checkout's sources failed, as it says above; the one an earlier build left isn't run: fix it, or run \`bun run build:compiler\` in ${checkout}`);
  }
  return compiler;
}

/** The React the project has installed, whose API the react crate is built
 * with (ADR 0043): what a later React added doesn't compile. `null` without
 * one, which gets the latest's. */
/**
 * The React release the project at `root` installed, or `null`: the one
 * answer, of a host's build and the playground's alike.
 * @param {string} root
 * @returns {string | null}
 */
export function installedReact(root) {
  try {
    const require = createRequire(join(root, "package.json"));
    return JSON.parse(readFileSync(require.resolve("react/package.json"), "utf8")).version;
  } catch {
    return null;
  }
}

/**
 * `@rust-js/runtime` and `@rust-js/std` as the project installed them, of
 * `version`, the compiler's: the helpers and std's bodies the JS it writes
 * imports are its release's (ADR 0103, the std-bodies decision). Throws
 * with what to install otherwise.
 */
function checkRuntime(root, version) {
  for (const name of ["runtime", "std"]) {
    // Where Node finds a package, from the app up: looked for each time, as
    // it may be installed while a dev server runs.
    let found;
    for (let dir = resolve(root); !found; dir = dirname(dir)) {
      const candidate = join(dir, "node_modules", "@rust-js", name, "package.json");
      if (existsSync(candidate)) found = candidate;
      else if (dirname(dir) === dir) break;
    }
    if (!found) throw new Error(`the JS rust-js ${version} writes imports @rust-js/${name} ${version}: install it`);
    const installed = JSON.parse(readFileSync(found, "utf8")).version;
    if (installed !== version) throw new Error(`rust-js ${version} needs @rust-js/${name} ${version}, not the installed ${installed}`);
  }
}

/** `command`'s stdout; what it says on stderr as it succeeds, its warnings,
 * go to `warn`, if given. */
function run(command, args, cwd, env = {}, warn) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, env: { ...process.env, RUST_JS_JS_RUNTIME: process.execPath, ...env }, stdio: ["ignore", "pipe", "pipe"] });
    let output = "";
    let errors = "";
    child.stdout.setEncoding("utf8").on("data", chunk => { output += chunk; });
    child.stderr.setEncoding("utf8").on("data", chunk => { errors += chunk; });
    child.on("error", reject);
    child.on("close", code => {
      if (code !== 0) return reject(new Error(errors || `${command} exited with ${code}`));
      if (warn && errors) warn(errors);
      resolve(output);
    });
  });
}

/**
 * What a host builds an app's Rust with (ADR 0101): `rustJs`, the compiler,
 * `findCompiler`'s, and Cargo, which checks each crate with it.
 * @param {{ root: string, rustJs: string }} options
 */
export function createNativeBuilder({ root, rustJs }) {
  const compilerPath = existsSync(rustJs) ? realpathSync(rustJs) : rustJs;
  const nativePackage = join(dirname(compilerPath), "../package.json");
  const packaged = existsSync(nativePackage) && JSON.parse(readFileSync(nativePackage, "utf8")).name === "@rust-js/native";
  const compilerCommand = packaged ? process.execPath : rustJs;
  const compilerArgs = packaged ? [compilerPath] : [];
  // What the compiler says it is, `--version-json`, asked once: which
  // runtime it needs, and which toolchain Cargo runs.
  let identity;
  const compilerIdentity = async () => (identity ??= parseCompilerIdentity(await run(compilerCommand, [...compilerArgs, "--version-json"], root)));
  // The Rust a Cargo build runs: the compiler's, as it says it was built
  // with; without a compiler, Cargo's own.
  const toolchain = async () => (existsSync(rustJs) ? (await compilerIdentity()).toolchain : undefined);
  /** A crate's checks of its JS as `files` are, a build's (ADR 0117): what
   * they say goes to `warn`; one that fails the build throws. */
  const checks = async ({ input, files, warn }) => {
    await run(compilerCommand, [...compilerArgs, "--checks", input, ...files], root, {}, warn);
  };

  const builder = {
    /** A Cargo workspace's check (ADR 0101), with the compiler's toolchain,
     * this compiler, and the React the project has installed. */
    async checkCargo(options) {
      const identity = await compilerIdentity();
      checkRuntime(root, identity.version);
      const react = installedReact(root) ?? undefined;
      try {
        return await checkCargo({ ...options, toolchain: identity.toolchain, compiler: rustJs, react });
      } catch (error) {
        if (react && error.message.includes("configured out")) {
          error.message += `\nnote: this project has React ${react}; an item gated \`react = "X.Y"\` needs React X.Y or later\n`;
        }
        throw error;
      }
    },
    /** The check an editor runs (ADR 0222), as `checkCargo`'s, its
     * messages printed as they come, and with `lints`, `rust-js clippy`'s
     * (ADR 0888); no JS in source, and no runtime needed. */
    async lintCheck(options) {
      if (!existsSync(rustJs)) throw new Error(`${options.lints ? "rust-js clippy" : "rust-js-check"}: ${withoutCompiler(rustJs)}`);
      return lintCheck({ ...options, toolchain: await toolchain(), compiler: rustJs, react: installedReact(root) ?? undefined });
    },
    /** The workspace of a Cargo manifest, and its target directory. */
    async cargoWorkspace(options) {
      return cargoWorkspace({ ...options, toolchain: await toolchain() });
    },
    /**
     * `rust-js build`'s Cargo build (ADR 0412): `checkCargo` of `options`,
     * each module's JS written beside its Rust, to be committed (ADR 0041),
     * or with `check`, compared with it; then each of the workspace's
     * crates' checks of its JS, a build's (ADR 0117), what they say given
     * to `warn`. What Cargo writes is out of what a bundler watches:
     * Vite's Tailwind takes a file of `target/` for a new one to scan, and
     * Turbopack each for a change.
     * @param {Omit<Parameters<typeof checkCargo>[0], "toolchain" | "compiler" | "react" | "inSource" | "targetDir"> & { warn?: (message: string) => void }} options
     */
    async build({ warn, ...given }) {
      const options = { ...given, inSource: true, targetDir: process.env.CARGO_TARGET_DIR ?? join(cacheDirOf(root), "target") };
      const { root: workspace } = await builder.cargoWorkspace({ manifestPath: options.manifestPath, offline: options.offline, targetDir: options.targetDir });
      const built = await builder.checkCargo(options);
      for (const [crate, manifest] of built.manifests) {
        // The app's own crates' checks, not an npm package's (ADR 0414).
        if (manifest.input.startsWith(workspace + sep) && !packageOf(options.packages ?? [], manifest.input)) {
          await checks({ input: manifest.input, files: built.files.get(crate) ?? [], warn });
        }
      }
      return { ...built, workspace };
    },
  };
  return builder;
}

/**
 * `rust-js build`, an app's Rust compiled once, by its own process beside
 * the app's dev server or build, whatever runs that (ADR 0412): `cargo
 * check` of the package `app`'s `Cargo.toml` is of, or of a workspace's,
 * `packageName`, a full-stack app's client say, each module's JS and
 * `.d.ts` written beside its Rust, to be committed (ADR 0041), and the
 * crate's checks of a build (ADR 0117), what they say given to `log`. With
 * `check`, nothing is written: a file that isn't what the Rust compiles to
 * is an error. A Next.js app's Pages Router routes get no declarations,
 * which it would take as routes (ADR 0276).
 * @param {{ app: string, packageName?: string, check?: boolean, log?: (line: string) => void }} options
 * @returns {Promise<void>}
 */
export async function buildApp({ app, packageName: named, check = false, log = (line) => process.stderr.write(`${line}\n`) }) {
  const { writePatch } = await import("./patch.js");
  // The app's Cargo, and its editor, find the crates its npm packages have
  // by a patch, which an install's `postinstall` writes, and again here:
  // an app of no `package.json` has none.
  const packages = existsSync(join(app, "package.json")) ? writePatch(app).map(({ name, dir }) => ({ name, dir })) : [];
  const compiler = await findCompiler(app);
  if (!existsSync(compiler)) throw new Error(`rust-js: ${withoutCompiler(compiler)}`);
  const builder = createNativeBuilder({ root: app, rustJs: compiler });
  const manifestPath = join(app, "Cargo.toml");
  if (!existsSync(manifestPath)) throw new Error(`rust-js: no Cargo.toml in ${app}: run \`rust-js build\` where the app's is`);
  const { own } = await builder.cargoWorkspace({ manifestPath });
  const packageName = named ?? own;
  if (!packageName) throw new Error(`rust-js: ${manifestPath} is a workspace's: name the app's package, \`rust-js build --package <name>\``);
  await builder.build({ manifestPath, packageName, check, routes: nextRoutes(app), packages, warn: (message) => log(message.trimEnd()) });
}

/**
 * `rust-js clippy` (ADR 0888): the app's Rust, `cargo check` of the package
 * `app`'s `Cargo.toml` is of, or `packageName`, linted by clippy's lints as
 * rust-js reads it, inside JSX, and what JS reads used (ADR 0440), with
 * `lints`, clippy's flags, `["-D", "warnings"]`. Nothing is written beside
 * the Rust, in a target directory of its own. Cargo's exit code.
 * @param {{ app: string, packageName?: string, lints: string[] }} options
 * @returns {Promise<number>}
 */
export async function clippyApp({ app, packageName, lints }) {
  const { writePatch } = await import("./patch.js");
  // Its crates where its npm packages have them, as a build's.
  if (existsSync(join(app, "package.json"))) writePatch(app);
  const manifestPath = join(app, "Cargo.toml");
  if (!existsSync(manifestPath)) throw new Error(`rust-js: no Cargo.toml in ${app}: run \`rust-js clippy\` where the app's is`);
  const builder = createNativeBuilder({ root: app, rustJs: await findCompiler(app) });
  const targetDir = process.env.CARGO_TARGET_DIR ?? join(cacheDirOf(app), "clippy");
  return builder.lintCheck({ manifestPath, packageName, lints, targetDir });
}

/** A Next.js app's Pages Router directories, where a file is a route, of
 * an app that depends on Next.js; none of another's. */
function nextRoutes(app) {
  const manifest = join(app, "package.json");
  if (!existsSync(manifest)) return [];
  const { dependencies = {}, devDependencies = {} } = JSON.parse(readFileSync(manifest, "utf8"));
  return "next" in dependencies || "next" in devDependencies ? [join(app, "pages"), join(app, "src/pages")] : [];
}
