// Native build preparation. Hosts provide scheduling and consume manifests.
import { spawn } from "node:child_process";
import { existsSync, readFileSync, realpathSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { cargoWorkspace, checkCargo, editorCheck, readsCargo } from "./cargo.js";
import { parseCompilerIdentity } from "./manifest.js";

/** This checkout, where a compiler built from it is. */
const checkout = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const defaultCompiler = join(checkout, "target/debug/rust-js");

/**
 * A rebuild of what a save touched, however many files it was: each change
 * asks for `items`, and one run at a time, after 30 ms more of a save's
 * events, gives `run` all asked for since, again while more are asked;
 * `after`, once none are. Asking returns the run, to wait for it. The one
 * way a host coalesces its rebuilds, Vite's and Next.js's alike.
 * @template T
 * @param {(batch: T[]) => Promise<void>} run
 * @param {{ after?: () => void, stopped?: () => boolean }} [options]
 * @returns {(...items: T[]) => Promise<void>}
 */
export function coalesced(run, { after = () => {}, stopped = () => false } = {}) {
  const pending = new Set();
  let active;
  const drain = async () => {
    while (pending.size && !stopped()) {
      await new Promise((done) => setTimeout(done, 30));
      const batch = [...pending];
      pending.clear();
      await run(batch);
    }
    after();
  };
  return (...items) => {
    for (const item of items) pending.add(item);
    return (active ??= drain().finally(() => { active = undefined; }));
  };
}

/** Where rust-js keeps what it makes for an app, outside what it commits:
 * its Cargo builds, and each host's own. */
export function cacheDirOf(root) {
  return join(root, "node_modules/.cache/rust-js");
}

/** What a host says without a compiler at `rustJs` (ADR 0041): that it
 * builds from `committed`, the JS committed beside the Rust, or, with none,
 * that it can't. */
export function withoutCompiler(rustJs, committed) {
  return committed
    ? `no rust-js at ${rustJs}: using the committed ${committed}. Install rust-js or build it with cargo build to compile the Rust.`
    : `no rust-js at ${rustJs}: install rust-js or build it with cargo build to compile the Rust`;
}

/**
 * The compiler a host runs for an app at `root`, the one answer every host
 * and command asks: `RUST_JS_COMPILER` where it names one, a test's or a
 * script's; else the app's `@rust-js/native`; else this checkout's build.
 */
export function findCompiler(root) {
  if (process.env.RUST_JS_COMPILER) return resolve(root, process.env.RUST_JS_COMPILER);
  const require = createRequire(join(root, "package.json"));
  try { return join(dirname(require.resolve("@rust-js/native/package.json")), "bin/rust-js"); }
  catch (error) {
    if (error.code !== "MODULE_NOT_FOUND") throw error;
    return defaultCompiler;
  }
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
 * `@rust-js/runtime` as the project installed it, of `version`, the
 * compiler's: the helpers the JS it writes imports are its release's (ADR
 * 0103). Throws with what to install otherwise.
 */
function checkRuntime(root, version) {
  // Where Node finds a package, from the app up: looked for each time, as
  // it may be installed while a dev server runs.
  let found;
  for (let dir = resolve(root); !found; dir = dirname(dir)) {
    const candidate = join(dir, "node_modules", "@rust-js", "runtime", "package.json");
    if (existsSync(candidate)) found = candidate;
    else if (dirname(dir) === dir) break;
  }
  if (!found) throw new Error(`the JS rust-js ${version} writes imports @rust-js/runtime ${version}: install it`);
  const installed = JSON.parse(readFileSync(found, "utf8")).version;
  if (installed !== version) throw new Error(`rust-js ${version} needs @rust-js/runtime ${version}, not the installed ${installed}`);
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
 * and Cargo, which checks each crate with it.
 * @param {{ root: string, rustJs?: string }} options
 */
export function createNativeBuilder({ root, rustJs = findCompiler(root) }) {
  const compilerPath = existsSync(rustJs) ? realpathSync(rustJs) : rustJs;
  const compilerInputs = [...new Set([rustJs, compilerPath])];
  const nativePackage = join(dirname(compilerPath), "../package.json");
  const packaged = existsSync(nativePackage) && JSON.parse(readFileSync(nativePackage, "utf8")).name === "@rust-js/native";
  if (packaged) {
    compilerInputs.push(nativePackage, join(dirname(compilerPath), "compiler"));
  }
  const compilerCommand = packaged ? process.execPath : rustJs;
  const compilerArgs = packaged ? [compilerPath] : [];
  // What the compiler says it is, `--version-json`: asked again when it's
  // replaced, as it may be while a dev server runs, which then checks the
  // runtime and picks the toolchain by the new one's answer.
  let identity;
  // Its files' sizes, times and inodes: what tells a compiler replaced from
  // one that ran, which macOS's `fs.watch` reports alike.
  const compilerKey = () => compilerInputs.map((path) => {
    const stat = statSync(path, { throwIfNoEntry: false });
    return stat ? `${stat.size}:${stat.mtimeMs}:${stat.ino}` : "missing";
  }).join(" ");
  const compilerIdentity = async () => {
    const key = compilerKey();
    if (identity?.key !== key) {
      identity = { key, value: parseCompilerIdentity(await run(compilerCommand, [...compilerArgs, "--version-json"], root)) };
    }
    return identity.value;
  };
  // The Rust a Cargo build runs: the compiler's, as it says it was built
  // with; without a compiler, Cargo's own, which says where a workspace is
  // for the committed JS's fallback.
  const toolchain = async () => (existsSync(rustJs) ? (await compilerIdentity()).toolchain : undefined);
  // What a change to makes a host build again: the compiler.
  const watchFiles = compilerInputs;
  /** Whether `file` is the compiler, replaced since it last said what it
   * is: an event on it a host builds again for, where one that only ran
   * it is none. */
  const compilerChanged = (file) => watchFiles.includes(file) && compilerKey() !== identity?.key;
  /** A crate's checks of its JS as `files` are, a save's or a build's (ADR
   * 0117): what they say goes to `warn`; one that fails the build throws. */
  const checks = async ({ input, files, save, warn }) => {
    await run(compilerCommand, [...compilerArgs, "--checks", input, "--hooks", save ? "save" : "build", ...files], root, {}, warn);
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
     * messages printed as they come; no JS in source, and no runtime needed. */
    async editorCheck(options) {
      if (!existsSync(rustJs)) throw new Error(`rust-js-check: ${withoutCompiler(rustJs)}`);
      return editorCheck({ ...options, toolchain: await toolchain(), compiler: rustJs, react: installedReact(root) ?? undefined });
    },
    /** The workspace of a Cargo manifest, and its target directory. */
    async cargoWorkspace(options) {
      return cargoWorkspace({ ...options, toolchain: await toolchain() });
    },
    /**
     * A Cargo build a host runs again as what it reads changes (ADR 0101),
     * Vite's plugin's and Next.js's: `build({ save, warn })` is `checkCargo`
     * of `options`, of the workspace its manifest is of, then each of the
     * workspace's crates' checks of its JS, a save's or a build's (ADR 0117),
     * what they say given to `warn`; and it keeps what it read.
     * `reads(file)` is whether a change to `file` is one to build again for,
     * the compiler, the workspace's Rust and manifests but not what Cargo
     * writes, or a source the last build read wherever it is; `sources()`
     * is each of those, to watch; and `committed()` is the JS of the
     * package's root module committed beside its Rust (ADR 0041), what a
     * host uses without a compiler, if there's one.
     * @param {Omit<Parameters<typeof checkCargo>[0], "toolchain" | "compiler" | "react">} options
     */
    cargoSession(given) {
      // What Cargo writes is out of what a host's bundler watches: Vite's
      // Tailwind, which takes a file of `target/` for a new one to scan, and
      // reloads the page, and Turbopack, which takes each for a change.
      // A host's JS is beside each module's Rust, to be committed (ADR 0041),
      // unless it's asked not to be.
      const options = { ...given, inSource: given.inSource ?? true, targetDir: process.env.CARGO_TARGET_DIR ?? join(cacheDirOf(root), "target") };
      let workspace, targetDir, manifests = [];
      return {
        /** @param {{ save?: boolean, warn?: (message: string) => void }} [run] */
        async build({ save = false, warn } = {}) {
          ({ root: workspace, target: targetDir } = await builder.cargoWorkspace({ manifestPath: options.manifestPath, offline: options.offline, targetDir: options.targetDir }));
          const built = await builder.checkCargo(options);
          manifests = [...built.manifests.values()];
          for (const [crate, manifest] of built.manifests) {
            if (manifest.input.startsWith(workspace + sep)) {
              await checks({ input: manifest.input, files: built.files.get(crate) ?? [], save, warn });
            }
          }
          return { ...built, workspace };
        },
        reads: (file) => compilerChanged(file)
          || (workspace !== undefined && readsCargo(file, { workspace, target: targetDir, manifests })),
        sources: () => manifests.flatMap((manifest) => manifest.sources),
        async committed() {
          const { libs } = await builder.cargoWorkspace({ manifestPath: options.manifestPath, offline: options.offline });
          const lib = libs.get(options.packageName);
          return lib && [".jsx", ".js"].map((extension) => lib.replace(/\.rs$/, extension)).find(existsSync);
        },
      };
    },
    watchFiles,
    compilerChanged,
  };
  return builder;
}
