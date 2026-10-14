// Compile Rust to ordinary JS/JSX files; Vite and plugin-react own bundling
// and Fast Refresh. The compiler manifest owns dependencies and output paths.
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile } from "node:fs/promises";
import { dirname, extname, join, resolve, sep } from "node:path";
import { cacheDirOf, createNativeBuilder, findCompiler, withoutCompiler } from "@rust-js/build/build";
import { parseManifest } from "@rust-js/build/manifest";
import { writePatch } from "@rust-js/build/patch";

/** The app Vite's `root` is of: the nearest directory with a `package.json`. */
function appDir(root) {
  for (let dir = root; ; dir = dirname(dir)) {
    if (existsSync(join(dir, "package.json"))) return dir;
    if (dirname(dir) === dir) return undefined;
  }
}

/**
 * An app's Rust, built with Cargo (ADR 0101): its one way. Cargo checks each
 * crate, with rust-js as its workspace wrapper, and the app imports
 * `package`'s JS as `rust-js:<package>`. It's beside the Rust, as a project
 * commits it (ADR 0041), unless `inSource` is `false`: then where Cargo's
 * build of it has it. `manifestPath` is relative to Vite's root,
 * `Cargo.toml` by default.
 * @param {object} options
 * @param {{ package: string, manifestPath?: string, features?: string[], noDefaultFeatures?: boolean, offline?: boolean, inSource?: boolean }} [options.cargo]
 * @param {string} [options.rustJs] Compiler binary; defaults to the app's native package, then this checkout.
 * @param {(job: { crate: string, output: string, manifest: string }) => Promise<void>} [options.compile]
 *   The playground's alone, which builds its own Rust with rust-js.wasm, the
 *   compiler it ships: `crates`, each written beside it with a manifest (ADR
 *   0042) at `manifest`, all paths absolute, or rustc's errors thrown.
 * @param {string[]} [options.crates] The playground's crate roots, relative to Vite's root.
 * @param {string} [options.cacheDir] Where the playground's manifests are kept.
 */
export default function rustJs({ cargo, rustJs, compile: custom, crates = [], cacheDir } = {}) {
  if (!cargo === !custom) {
    throw new Error("rust-js: give `cargo`, the app's Cargo package (ADR 0101); `compile` is the playground's own build");
  }
  if (cargo) crates = [cargo.package];
  let root, server, builder, logger, closed = false;
  // A Cargo build's: the package's JS, the manifest Vite is given, the
  // workspace it's of, and its session, which says what a build read.
  let entry, workspaceManifest, workspace, session;
  let active;
  const pending = new Set();
  const manifests = new Map();
  const failures = new Map();
  const aliases = new Map();
  const maps = new Set();
  // The modules Cargo's builds have, outside the app.
  const generated = new Set();
  // Committed files used without rust-js, whose maps may not be committed.
  const committed = new Set();
  const manifestPath = crate => join(cacheDir ?? cacheDirOf(root), "vite", createHash("sha256").update(resolve(root, crate)).digest("hex") + ".json");

  // Every crate Cargo built, or found done: each is a manifest here, and
  // every edit is one `cargo check`, which rebuilds what it changed.
  async function compileCargo() {
    const old = JSON.stringify([...manifests.values()].flatMap(m => m.artifacts.map(a => a.file)).sort());
    // A member's manifest is of the workspace its siblings are of too.
    // The Vite server's is a save's, whose checks are those not only for a
    // build; what rust-js warns of, a check's, is Vite's (ADR 0117).
    const built = await session.build({ save: Boolean(server), warn: message => logger.warn(message.trimEnd(), { timestamp: true }) });
    workspace = built.workspace;
    server?.watcher.add(workspace);
    manifests.clear();
    for (const [name, manifest] of built.manifests) manifests.set(name, manifest);
    entry = built.js;
    failures.clear();
    // Its imports name each module's file, `.jsx` or `.js`: no aliases.
    maps.clear();
    generated.clear();
    for (const current of manifests.values()) server?.watcher.add(current.sources);
    // Each module's JS where it's served from: beside its Rust, or Cargo's.
    const files = [...built.files.values()].flat();
    server?.watcher.add(files);
    for (const file of files) {
      generated.add(file);
      maps.add(`${file}.map`);
    }
    // Another build of a crate, of other features say, is another module ID.
    const now = JSON.stringify([...manifests.values()].flatMap(m => m.artifacts.map(a => a.file)).sort());
    if (server && old !== "[]" && old !== now) {
      server.moduleGraph.invalidateAll();
      server.ws.send({ type: "full-reload" });
    }
  }

  async function compile(crate) {
    if (cargo) return compileCargo();
    const manifest = manifestPath(crate);
    await mkdir(dirname(manifest), { recursive: true });
    const output = crate.replace(/\.rs$/, ".js");
    await custom({ crate: resolve(root, crate), output: resolve(root, output), manifest });
    const result = parseManifest(await readFile(manifest, "utf8"));
    const old = manifests.get(crate);
    manifests.set(crate, result);
    failures.delete(crate);
    server?.watcher.add(result.sources);
    aliases.clear();
    maps.clear();
    for (const current of manifests.values()) {
      for (const module of current.modules) {
        if (module.map) maps.add(resolve(root, module.map));
        const file = resolve(root, module.file);
        const stem = file.slice(0, -extname(file).length);
        aliases.set(stem + ".js", file);
        aliases.set(stem + ".jsx", file);
      }
    }
    // A suffix or module-set change invalidates old Vite module IDs. Ordinary
    // edits retain their IDs and use plugin-react's state-preserving refresh.
    if (server && old && JSON.stringify(old.artifacts.map(a => a.file)) !== JSON.stringify(result.artifacts.map(a => a.file))) {
      server.moduleGraph.invalidateAll();
      server.ws.send({ type: "full-reload" });
    }
  }

  async function drain() {
    while (pending.size && !closed) {
      // Let filesystem events from a single save accumulate before spawning.
      await new Promise(resolve => setTimeout(resolve, 30));
      const batch = [...pending];
      pending.clear();
      for (const crate of batch) {
        try { await compile(crate); }
        catch (error) { failures.set(crate, error.message); }
      }
    }
    if (server && !closed) {
      if (failures.size) {
        const message = [...failures.values()].join("\n");
        server.config.logger.error(message, { timestamp: true });
        server.ws.send({ type: "error", err: { message, stack: "", plugin: "rust-js" } });
      } else {
        // Vite clears its error overlay on an update, even when correcting an
        // error reproduces identical JS and therefore triggers no file update.
        server.ws.send({ type: "update", updates: [] });
      }
    }
  }

  function schedule(affected) {
    for (const crate of affected) pending.add(crate);
    if (!active) active = drain().finally(() => { active = undefined; });
    return active;
  }
  function changed(event, path) {
    if (closed) return;
    const file = resolve(path);
    const common = builder?.watchFiles.includes(file);
    if (cargo) {
      if (session?.reads(file)) void schedule(crates);
      return;
    }
    if (!common && !file.endsWith(".rs")) return;
    const affected = crates.filter(crate => {
      if (common || manifests.get(crate)?.sources.includes(file) || resolve(root, crate) === file) return true;
      // Newly declared or missing modules aren't in the last successful
      // manifest. Retry roots under whose source tree the edit occurred.
      return failures.has(crate) || ((event === "add" || !manifests.has(crate)) && file.startsWith(dirname(resolve(root, crate)) + sep));
    });
    void schedule(affected);
  }

  return {
    name: "rust-js",
    enforce: "pre",
    configResolved(config) {
      root = config.root;
      logger = config.logger;
      if (cargo) {
        workspaceManifest = resolve(root, cargo.manifestPath ?? "Cargo.toml");
        workspace = dirname(workspaceManifest);
      }
      if (cargo) {
        rustJs = resolve(root, rustJs ?? findCompiler(root));
        builder = createNativeBuilder({ root, rustJs });
        session = builder.cargoSession({
          manifestPath: workspaceManifest, packageName: cargo.package,
          features: cargo.features, noDefaultFeatures: cargo.noDefaultFeatures, offline: cargo.offline,
          inSource: cargo.inSource ?? true,
        });
      }
    },
    async buildStart() {
      // The app's Cargo, and its editor, find the crates its npm packages
      // have by a patch, which an install's `postinstall` writes, and Vite
      // again as it starts: a crate installed twice stops it.
      const app = appDir(root);
      if (app) {
        try {
          writePatch(app);
        } catch (error) {
          this.error(`rust-js: ${error instanceof Error ? error.message : error}`);
        }
      }
      // The generated JS is committed, as ReScript recommends (ADR 0041), so
      // a checkout without rust-js still builds from it. With rust-js, the
      // Rust is always compiled, and an error is never hidden by an old file.
      // A Cargo workspace's is beside each crate's Rust (ADR 0041).
      if (cargo && !existsSync(rustJs)) {
        const file = await session.committed();
        if (!file) this.error(withoutCompiler(rustJs));
        entry = file;
        committed.add(file);
        generated.add(file);
        this.warn(withoutCompiler(rustJs, file));
        return;
      }
      await schedule(crates);
      if (failures.size) {
        const message = [...failures.values()].join("\n");
        if (server) this.warn(message);
        else this.error(message);
      }
      for (const manifest of manifests.values()) for (const file of manifest.sources) this.addWatchFile(file);
    },
    // A committed file names its source map, which isn't committed: without
    // it, drop the comment rather than have Vite report a missing file.
    async load(id) {
      const file = id.split("?")[0];
      if (!committed.has(file) || existsSync(`${file}.map`)) return;
      return (await readFile(file, "utf8")).replace(/\/\/# sourceMappingURL=\S+\s*$/, "");
    },
    resolveId(source, importer, options) {
      if (cargo && source === `rust-js:${cargo.package}`) return entry;
      // A package a module Cargo built imports is the app's, where the app
      // has it installed: resolved as from Vite's root, not the target's.
      if (importer && generated.has(importer.split("?")[0]) && !source.startsWith(".") && !source.startsWith("/")) {
        return this.resolve(source, join(root, "index.html"), { ...options, skipSelf: true });
      }
      // What a committed module imports of the crates' is theirs too: found
      // from the entry, when there's no rust-js to say what they are.
      if (cargo && importer && committed.has(importer.split("?")[0]) && source.startsWith(".")) {
        const file = resolve(dirname(importer.split("?")[0]), source);
        if (/\.jsx?$/.test(file) && existsSync(file)) {
          committed.add(file);
          generated.add(file);
        }
      }
      if (!importer || !source.startsWith(".")) return;
      return aliases.get(resolve(dirname(importer.split("?")[0]), source));
    },
    // Neither a `.rs` file nor a source map written from one is a module: the
    // JS compiled from them brings its own update. Pass on only what depends
    // on a `.rs` file as a plain file, like a stylesheet holding its Tailwind
    // classes, which then updates in place. Otherwise Tailwind reloads the
    // page, as it does for a template file it scans.
    hotUpdate({ file, modules }) {
      if (maps.has(file)) return [];
      if (!file.endsWith(".rs")) return;
      return [...new Set(modules.flatMap(module => [...module.importers]))];
    },
    async handleHotUpdate() {
      // Native publication replaces files individually. Let Vite read the
      // completed module set, including its manifest, before sending updates.
      await active;
    },
    configureServer(value) {
      server = value;
      server.watcher.add([...(builder?.watchFiles ?? []), ...(cargo ? [workspace] : crates.map(crate => dirname(resolve(root, crate))))]);
      server.watcher.on("all", changed);
    },
    async closeBundle() {
      if (!server) return;
      closed = true;
      server?.watcher.off("all", changed);
      await active;
    },
    async closeWatcher() {
      closed = true;
      await active;
    },
  };
}
