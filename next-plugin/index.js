// Compile a Next.js app's Rust to ordinary JS/JSX files beside it, as the
// Vite plugin's Cargo builds do (ADR 0101): Next.js, and Turbopack's Fast
// Refresh, see them as the app's own. A Rust route, `app/page.rs`, is then
// `app/page.jsx`, a route as Next.js finds one (ADR 0192).
import { spawn } from "node:child_process";
import { existsSync, watch } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve, sep } from "node:path";
import { coalesced, createNativeBuilder, findCompiler, withoutCompiler } from "@rust-js/build/build";
import { writePatch } from "@rust-js/build/patch";

/**
 * Run `next <command>` in `app`, its Rust compiled first, and for `dev`
 * again on every save, which goes on through a failure, as Next.js's own
 * does. What's left is Next.js's: its arguments, and its exit code.
 * @param {{ app: string, command: string, args: string[], rustJs?: string, log?: (line: string) => void }} options
 * @returns {Promise<number>}
 */
export async function run({ app, command, args, rustJs, log = (line) => process.stderr.write(`${line}\n`) }) {
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
  } else if (!(await compile()) && command !== "dev") return 1;
  // Its Rust's JS alone, which an app's build may read before Next.js runs.
  if (command === "compile") return 0;

  // What Cargo reads of the crate, its Rust and its manifests, and what
  // rust-js reads of its own, but not what Cargo writes.
  const watchers = [];
  if (command === "dev" && compiling) {
    // A save's events, together, are one compile.
    const rebuild = coalesced(async () => {
      if (await compile()) log("rust-js: compiled");
    }, { after: () => watchOutside() });
    const changed = (file) => {
      if (session.reads(file)) void rebuild(true);
    };
    watchers.push(watch(workspace, { recursive: true }, (_, name) => name && changed(resolve(workspace, name))));
    for (const file of builder.watchFiles) if (existsSync(file)) watchers.push(watch(file, () => changed(file)));
    // Each source rust-js read outside the workspace, a module by `#[path]`
    // or a file an `include_str!` reads, as the last build has them.
    const outside = new Set();
    const watchOutside = () => {
      for (const file of session.sources()) {
        if (file.startsWith(workspace + sep) || outside.has(file) || !existsSync(file)) continue;
        outside.add(file);
        watchers.push(watch(file, () => changed(file)));
      }
    };
    watchOutside();
  }

  const next = createRequire(join(app, "package.json")).resolve("next/dist/bin/next");
  const child = spawn(process.execPath, [next, command, ...args], { cwd: app, stdio: "inherit" });
  const stop = (signal) => child.kill(signal);
  process.on("SIGINT", stop);
  process.on("SIGTERM", stop);
  return new Promise((done) => {
    child.on("exit", (code, signal) => {
      for (const watcher of watchers) watcher.close();
      process.off("SIGINT", stop);
      process.off("SIGTERM", stop);
      done(code ?? (signal ? 1 : 0));
    });
  });
}
