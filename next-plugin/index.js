// Compile a Next.js app's Rust to ordinary JS/JSX files beside it, as the
// Vite plugin's Cargo builds do (ADR 0101): Next.js, and Turbopack's Fast
// Refresh, see them as the app's own. A Rust route, `app/page.rs`, is then
// `app/page.jsx`, a route as Next.js finds one (ADR 0192). The app's
// config says so, `withRustJs(nextConfig)`, and whatever starts Next.js
// starts it (ADR 0380).
import { existsSync, watch } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve, sep } from "node:path";
import { coalesced, createNativeBuilder, findCompiler, withoutCompiler } from "@rust-js/build/build";
import { writePatch } from "@rust-js/build/patch";

const log = (line) => process.stderr.write(`${line}\n`);

/**
 * `nextConfig`, a Next.js app's config, as it is, its app's Rust compiled
 * by the phase Next.js loads it in: developing, once and again on each
 * save; building, once before Next.js builds; else not at all. `next dev`,
 * `next build`, a custom server and react.dev's `next-remote-watch` each
 * load it, so the app's scripts stay its own. The app is the directory
 * Next.js runs in.
 * @template C
 * @param {C} nextConfig an object, or Next.js's function of the phase
 * @returns {(phase: string, context: object) => Promise<object>}
 */
export function withRustJs(nextConfig) {
  return async (phase, context) => {
    await compiled(phase, process.cwd());
    return typeof nextConfig === "function" ? nextConfig(phase, context) : nextConfig;
  };
}

// Each phase's work in an app, once in a process however often Next.js
// loads its config, twice in one `next dev`'s or `next build`'s.
const started = new Map();
function compiled(phase, app) {
  // The phases, as Next.js names them.
  const { PHASE_DEVELOPMENT_SERVER, PHASE_PRODUCTION_BUILD } = createRequire(join(app, "package.json"))("next/constants");
  const command = { [PHASE_DEVELOPMENT_SERVER]: "dev", [PHASE_PRODUCTION_BUILD]: "build" }[phase];
  if (!command) return;
  const key = `${command} ${app}`;
  if (!started.has(key)) started.set(key, run({ app, command }));
  return started.get(key);
}

/**
 * The app's Rust's JS, written beside it, and nothing of Next.js's: what
 * `rust-js-next compile` writes, for what reads it before Next.js runs.
 * @param {{ app: string, rustJs?: string }} options
 * @returns {Promise<void>}
 */
export function compile({ app, rustJs }) {
  return run({ app, command: "compile", rustJs });
}

/**
 * Compile `app`'s Rust for `command`: `dev`, now and again on every save,
 * on through a failure, as Next.js's own dev server goes on; `build` and
 * `compile`, once, a failure thrown, rustc's message.
 * @param {{ app: string, command: "dev" | "build" | "compile", rustJs?: string }} options
 */
async function run({ app, command, rustJs }) {
  // The app's Cargo, and its editor, find the crates its npm packages have
  // by a patch, which an install's `postinstall` writes, and again here.
  writePatch(app);
  const compiler = resolve(app, rustJs ?? findCompiler(app));
  const builder = createNativeBuilder({ root: app, rustJs: compiler });
  const manifestPath = join(app, "Cargo.toml");
  // The app's Rust is the package its `Cargo.toml` is of.
  const { root: workspace, own: packageName } = await builder.cargoWorkspace({ manifestPath });
  if (!packageName) throw new Error(`rust-js: ${manifestPath} is of no package; the app's Rust is one`);

  // One `cargo check` of the whole crate, which writes each module's JS
  // beside its Rust (ADR 0041), or an error, rustc's. Its manifests say
  // what it read, a source outside the workspace too. A Pages Router
  // route's directory gets no declarations, which Turbopack would take as
  // routes (ADR 0276).
  const session = builder.cargoSession({ manifestPath, packageName, routes: [join(app, "pages"), join(app, "src/pages")] });
  async function compile() {
    try {
      // The dev server's builds are a save's, whose checks are those not
      // only for a build (ADR 0117); what they say is the log's.
      await session.build({ save: command === "dev", warn: (message) => log(message.trimEnd()) });
      return true;
    } catch (error) {
      log(error instanceof Error ? error.message : String(error));
      return false;
    }
  }

  // The JS is committed, as ReScript recommends (ADR 0041), so a checkout
  // without rust-js still builds and serves from it, as Vite's does. With
  // rust-js, the Rust is always compiled, and an error never hidden by an
  // old file.
  const compiling = existsSync(compiler);
  if (!compiling) {
    const file = command === "compile" ? undefined : await session.committed();
    if (!file) throw new Error(`rust-js: ${withoutCompiler(compiler)}`);
    log(`rust-js: ${withoutCompiler(compiler, file)}`);
  } else if (!(await compile()) && command !== "dev") throw new Error("rust-js: the app's Rust doesn't compile");

  // What Cargo reads of the crate, its Rust and its manifests, and what
  // rust-js reads of its own, but not what Cargo writes: watched as long
  // as the dev server runs, which then stops them.
  if (command === "dev" && compiling) {
    // A save's events, together, are one compile.
    const rebuild = coalesced(async () => {
      if (await compile()) log("rust-js: compiled");
    }, { after: () => watchOutside() });
    const changed = (file) => {
      if (session.reads(file)) void rebuild(true);
    };
    watch(workspace, { recursive: true }, (_, name) => name && changed(resolve(workspace, name))).unref();
    for (const file of builder.watchFiles) if (existsSync(file)) watch(file, () => changed(file)).unref();
    // Each source rust-js read outside the workspace, a module by `#[path]`
    // or a file an `include_str!` reads, as the last build has them.
    const outside = new Set();
    const watchOutside = () => {
      for (const file of session.sources()) {
        if (file.startsWith(workspace + sep) || outside.has(file) || !existsSync(file)) continue;
        outside.add(file);
        watch(file, () => changed(file)).unref();
      }
    };
    watchOutside();
  }
}
