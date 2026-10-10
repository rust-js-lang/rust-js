// Compile a Next.js app's Rust to ordinary JS/JSX files beside it, as the
// Vite plugin's Cargo builds do (ADR 0101): Next.js, and Turbopack's Fast
// Refresh, see them as the app's own. A Rust route, `app/page.rs`, is then
// `app/page.jsx`, a route as Next.js finds one (ADR 0192).
import { spawn } from "node:child_process";
import { existsSync, watch } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve, sep } from "node:path";
import { createNativeBuilder, findCompiler } from "@rust-js/build/build";
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
  // What Cargo writes is out of what Turbopack watches: it watches the app's
  // files, all but node_modules' and .next's, and a `cargo check` on every
  // save rewrites many of target/'s, which it would see as many changes.
  process.env.CARGO_TARGET_DIR ??= join(app, "node_modules/.cache/rust-js/target");
  const compiler = resolve(app, rustJs ?? findCompiler(app));
  if (!existsSync(compiler)) throw new Error(`rust-js: no compiler at ${compiler}`);
  const builder = createNativeBuilder({ root: app, rustJs: compiler });
  const manifestPath = join(app, "Cargo.toml");
  const { root: workspace, target, packages } = await builder.cargoWorkspace({ manifestPath });
  const [packageName, ...others] = packages.keys();
  if (others.length) throw new Error(`rust-js: ${manifestPath} has several packages; the app's Rust is one`);

  // One `cargo check` of the whole crate, which writes each module's JS
  // beside its Rust (ADR 0041), or an error, rustc's.
  async function compile() {
    try {
      // A Pages Router route's directory gets no declarations, which
      // Turbopack would take as routes (ADR 0276).
      const routes = [join(app, "pages"), join(app, "src/pages")];
      await builder.checkCargo({ manifestPath, packageName, inSource: true, routes });
      return true;
    } catch (error) {
      log(error instanceof Error ? error.message : String(error));
      return false;
    }
  }

  if (!(await compile()) && command !== "dev") return 1;
  // Its Rust's JS alone, which an app's build may read before Next.js runs.
  if (command === "compile") return 0;

  // What Cargo reads of the crate, its Rust and its manifests, and what
  // rust-js reads of its own, but not what Cargo writes.
  const watchers = [];
  if (command === "dev") {
    let pending = false;
    let active;
    const drain = async () => {
      while (pending) {
        // A save's events, together, are one compile.
        await new Promise((done) => setTimeout(done, 30));
        pending = false;
        if (await compile()) log("rust-js: compiled");
      }
    };
    const changed = (file) => {
      if (!/\.rs$|(^|[\\/])Cargo\.(toml|lock)$/.test(file) || file.startsWith(target + sep)) return;
      pending = true;
      active ??= drain().finally(() => {
        active = undefined;
      });
    };
    watchers.push(watch(workspace, { recursive: true }, (_, name) => name && changed(resolve(workspace, name))));
    for (const file of builder.watchFiles) if (existsSync(file)) watchers.push(watch(file, () => changed(file)));
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
