// React's port, rust-js-lang/react-in-rust, at the commit
// `ports/react-in-rust.pin` names, built against this checkout: what a
// change here does to the port, seen before its porter's next regeneration.
//
//   bun scripts/react-in-rust.ts check   `cargo check` of its workspace: its
//                                        Rust against the bindings here, each
//                                        push that changes them (react-in-rust.yml)
//   bun scripts/react-in-rust.ts regen [crate ..]
//                                        its workspace, or the crates named,
//                                        compiled to JS by this compiler in
//                                        one `rust-js build`, against the JS
//                                        the pin committed (the nightly's `port`)
//
// The port is fetched at the pin into a directory of its own, beside a
// `rust-js` that is this checkout, as its Cargo.toml and package.json name
// `../rust-js`; never a porter's checkout. `regen` fails on a compile error
// or any file that differs, and lists them with a short diff, all of it in
// `target/react-in-rust.diff`. A difference that's intended is accepted by
// moving the pin to the port's commit that has it.
//
// Each checkout's directory is bounded (#914): one whose checkout is gone
// goes, the least recently used past `portsCap` go, and none is kept below
// the disk's floor (`bound`); its build, under the port's
// `node_modules/.cache/rust-js`, is of one build of the compiler, as
// `rust-js build` keeps every app's (`holdTarget`, #1057), and keeps no
// incremental cache, as each crate is compiled whole again whenever the
// compiler changes.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { appendFileSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, rmSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { floorBytes } from "../test/bounded";
import { bytesOf, evict, freeBytes, type Kept } from "../tooling/bounded.js";
import { cacheDirOf } from "../tooling/build.js";
import { channel } from "./toolchain";

const root = resolve(import.meta.dir, "..");
const repository = "https://github.com/rust-js-lang/react-in-rust.git";
const pinned = readFileSync(join(root, "ports/react-in-rust.pin"), "utf8")
  .split("\n")
  .map((line) => line.trim())
  .find((line) => line && !line.startsWith("#"));
if (!pinned || !/^[0-9a-f]{40}$/.test(pinned)) throw new Error("ports/react-in-rust.pin names no commit: a full SHA on a line of its own");
const pin: string = pinned;
const ports = process.env.RUNNER_TEMP ?? tmpdir();
const home = join(ports, `react-in-rust-${createHash("sha256").update(root).digest("hex").slice(0, 8)}`);
const port = join(home, "react-in-rust");
const compiler = resolve(process.env.RUST_JS_COMPILER ?? join(root, "target/debug/rust-js"));

/** The most every checkout's directory keeps together: one is about
 * 330 MB after a regen, the port's checkout and one build's crates. */
export const portsCap = 4 * 2 ** 30;

/** The checkouts' directories in `dir` bounded, but `own`, this run's:
 * each whose checkout is gone, its `rust-js` a link to nothing, or with no
 * link an hour after it was made; then, of those not used in the last
 * hour, as another run may be using it, the least recently used, until
 * they're under `cap` with `own`, and the disk's `free` space is above its
 * floor. What it removed. */
export function bound({ dir = ports, own = home, cap = portsCap, free = () => freeBytes(dir) }: { dir?: string; own?: string; cap?: number; free?: () => number } = {}): string[] {
  const hour = Date.now() - 60 * 60 * 1000;
  const removed: string[] = [];
  const kept: Kept[] = [];
  let names: string[] = [];
  try {
    names = readdirSync(dir).filter((name) => /^react-in-rust-[0-9a-f]{8}$/.test(name));
  } catch {}
  for (const name of names) {
    const at = join(dir, name);
    if (at === own) continue;
    const link = join(at, "rust-js");
    let made: number;
    try {
      made = statSync(at).mtimeMs;
    } catch {
      continue;
    }
    const linked = (() => {
      try {
        return lstatSync(link).isSymbolicLink();
      } catch {
        return false;
      }
    })();
    if (linked ? !existsSync(link) : made < hour) {
      rmSync(at, { recursive: true, force: true });
      removed.push(at);
      continue;
    }
    const used = (() => {
      try {
        return statSync(join(at, "used")).mtimeMs;
      } catch {
        return made;
      }
    })();
    if (used < hour) kept.push({ path: at, bytes: bytesOf(at), used });
  }
  const ownBytes = bytesOf(own);
  const before = new Set(kept.map((thing) => thing.path));
  evict(kept, { cap: Math.max(0, cap - ownBytes), free, floor: floorBytes });
  for (const path of before) if (!existsSync(path)) removed.push(path);
  return removed;
}

function run(cmd: string[], cwd: string, env: Record<string, string> = {}) {
  const p = spawnSync(cmd[0], cmd.slice(1), { cwd, env: { ...process.env, ...env }, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
  return { ok: p.status === 0, out: `${p.stdout ?? ""}${p.stderr ?? ""}`, stdout: p.stdout ?? "" };
}
function must(cmd: string[], cwd: string, env: Record<string, string> = {}) {
  const result = run(cmd, cwd, env);
  if (!result.ok) throw new Error(`${cmd.join(" ")} failed:\n${result.out.slice(-6000)}`);
  return result.stdout;
}
/** What a person reads of the run: here, and in the job's summary. */
function report(markdown: string) {
  console.log(markdown);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${markdown}\n`);
}

/** The port at the pin, as committed, beside this checkout. */
function fetch() {
  const removed = bound();
  if (removed.length) console.log(`react-in-rust: removed ${removed.length} other checkouts' directories, gone or past the cap`);
  mkdirSync(home, { recursive: true });
  // When it was last used, which `bound` reads.
  writeFileSync(join(home, "used"), "");
  const link = join(home, "rust-js");
  if (!existsSync(link) || realpathSync(link) !== realpathSync(root)) {
    rmSync(link, { force: true });
    symlinkSync(root, link);
  }
  if (!existsSync(join(port, ".git"))) must(["git", "init", "-q", port], home);
  // Its remote, which a run stopped between the two lost: added, or set
  // where it names another.
  const remote = run(["git", "remote", "get-url", "origin"], port);
  if (!remote.ok) must(["git", "remote", "add", "origin", repository], port);
  else if (remote.stdout.trim() !== repository) must(["git", "remote", "set-url", "origin", repository], port);
  if (run(["git", "rev-parse", "HEAD"], port).stdout.trim() !== pin) {
    must(["git", "fetch", "-q", "--depth=1", "origin", pin], port);
  }
  must(["git", "checkout", "-q", "-f", pin], port);
  // What the last run wrote goes; its builds, under node_modules, stay.
  must(["git", "clean", "-q", "-fdx", "-e", "node_modules"], port);
  // What rust-js reads of an install, as its `yarn install` links them:
  // each package of its workspaces by its name, which another imports it by
  // (`'shared/ReactSymbols'`, rd349) and whose `react` says React's version,
  // and the runtime and std the JS imports, this checkout's.
  const workspaces = (JSON.parse(readFileSync(join(port, "package.json"), "utf8")).workspaces as string[]).flatMap((glob) => {
    if (!glob.endsWith("/*")) throw new Error(`the port's workspace ${glob} isn't a directory's packages, \`dir/*\``);
    const dir = join(port, glob.slice(0, -2));
    return readdirSync(dir)
      .map((entry) => join(dir, entry))
      .filter((at) => existsSync(join(at, "package.json")))
      .map((at) => [JSON.parse(readFileSync(join(at, "package.json"), "utf8")).name as string, at]);
  });
  for (const [name, target] of [...workspaces, ["@rust-js/std", join(root, "std")]]) {
    const at = join(port, "node_modules", name);
    mkdirSync(dirname(at), { recursive: true });
    rmSync(at, { force: true, recursive: false });
    symlinkSync(target, at);
  }
}

/** The port's Prettier and its config's plugin, at the versions its
 * yarn.lock has, where rust-js finds them: the JS it commits is laid out
 * by them (ADR 0381), and without them rust-js's own layout differs in
 * every file. Installed beside the port, not by its whole `yarn install`. */
function layout() {
  const lock = readFileSync(join(port, "yarn.lock"), "utf8");
  const wanted = JSON.parse(readFileSync(join(port, "package.json"), "utf8")).devDependencies as Record<string, string>;
  const dependencies: Record<string, string> = {};
  for (const name of ["prettier", "prettier-plugin-hermes-parser"]) {
    const range = `${name}@${wanted[name]}`;
    const header = lock.split("\n").findIndex((line) => !line.startsWith(" ") && line.replace(/"/g, "").replace(/:$/, "").split(", ").includes(range));
    const version = lock.split("\n")[header + 1]?.match(/^ {2}version "([^"]+)"/)?.[1];
    if (header < 0 || !version) throw new Error(`the port's yarn.lock has no ${range}`);
    dependencies[name] = version;
  }
  const tools = join(home, "layout");
  mkdirSync(tools, { recursive: true });
  const manifest = JSON.stringify({ private: true, dependencies }, null, 2);
  if (!existsSync(join(tools, "package.json")) || readFileSync(join(tools, "package.json"), "utf8") !== manifest) {
    writeFileSync(join(tools, "package.json"), manifest);
    must(["bun", "install", "--silent"], tools);
  }
  for (const name of Object.keys(dependencies)) {
    const at = join(port, "node_modules", name);
    rmSync(at, { force: true, recursive: false });
    symlinkSync(join(tools, "node_modules", name), at);
  }
}

/** Its workspace's crates that have JS, by name: a binding crate's is the
 * library's it binds (ADR 0414). */
function members(): string[] {
  const metadata = JSON.parse(must(["cargo", `+${channel}`, "metadata", "--no-deps", "--format-version=1", "--locked"], port));
  return (metadata.packages as { name: string; metadata?: { "rust-js"?: { bindings?: boolean } } }[])
    .filter((p) => !p.metadata?.["rust-js"]?.bindings)
    .map((p) => p.name)
    .sort();
}

const [command] = process.argv.slice(2);
if (!import.meta.main) {
  // Imported, by test/react-in-rust.test.ts: nothing's run.
} else if (command === "check") {
  fetch();
  const target = join(port, "node_modules/.cache/check");
  const checked = run(["cargo", `+${channel}`, "check", "--workspace", "--locked", "--target", "wasm32-unknown-unknown", "--target-dir", target, "--message-format=short"], port);
  const errors = checked.out.split("\n").filter((line) => /\berror(\[E\d+\])?:/.test(line));
  if (!checked.ok) {
    report(`### react-in-rust at ${pin.slice(0, 9)} doesn't compile against this checkout's bindings\n\n\`\`\`\n${errors.join("\n").slice(0, 8000)}\n\`\`\`\n\nA binding's change broke the port: fix the binding, or tell its porter and move \`ports/react-in-rust.pin\` once the port has followed.`);
    process.exit(1);
  }
  report(`react-in-rust at ${pin.slice(0, 9)}: its workspace compiles against this checkout's bindings.`);
} else if (command === "regen") {
  if (!existsSync(compiler)) throw new Error(`no compiler at ${compiler}: \`bun run build\`, or set RUST_JS_COMPILER`);
  fetch();
  layout();
  // One build, as its porter's `yarn rust` is: the workspace's members at
  // once, or those named, each crate checked once and Cargo's on every core.
  const named = process.argv.slice(3);
  const crates = members();
  const unknown = named.filter((name) => !crates.includes(name));
  if (unknown.length) throw new Error(`react-in-rust has no crate with JS named ${unknown.join(", ")}: ${crates.join(", ")}`);
  // Without rustc's incremental cache, 420 of a build's 540 MB, which no
  // regen reads: after a change to the compiler each crate is compiled
  // whole again, and with none Cargo finds each one done.
  const built = run(["node", join(root, "tooling/cli.js"), "build", ...named.flatMap((name) => ["--package", name])], port, { RUST_JS_COMPILER: compiler, CARGO_INCREMENTAL: "0" });
  // Nothing kept below the disk's floor.
  if (freeBytes(port) < floorBytes) rmSync(cacheDirOf(port), { recursive: true, force: true });
  const failed = built.ok ? [] : [`\`\`\`\n${built.out.trim().split("\n").slice(-80).join("\n")}\n\`\`\``];
  // Every file the build changed or added, as git sees it.
  const changed = run(["git", "status", "--porcelain=v1", "--untracked-files=all"], port).stdout.split("\n").filter(Boolean);
  run(["git", "add", "-A", "--intent-to-add"], port);
  const diff = run(["git", "diff", "--no-color"], port).stdout;
  mkdirSync(join(root, "target"), { recursive: true });
  writeFileSync(join(root, "target/react-in-rust.diff"), diff);
  if (failed.length) report(`### react-in-rust at ${pin.slice(0, 9)} doesn't compile\n\n${failed.join("\n\n")}`);
  if (changed.length) {
    // Each file's first lines of change, so a person can tell a regression
    // from an intended change; the whole diff is the artifact's.
    const files = diff.split(/^(?=diff --git )/m).filter(Boolean);
    const shown = files.slice(0, 30).map((file) => file.split("\n").slice(0, 40).join("\n")).join("\n");
    // The compiler's commits since the rust-js the pin's JS was made on, its
    // subject's `Regenerate on rust-js <sha>`: each change above is one of
    // theirs, an intended one named by its ADR or React port gap, `rd313`.
    const subject = run(["git", "log", "-1", "--format=%s", pin], port).stdout;
    const made = subject.match(/^Regenerate on rust-js ([0-9a-f]{7,40})\b/)?.[1];
    const since = made && run(["git", "log", "--format=- %h %s", `${made}..HEAD`, "--", "src", "builtins", "webapi", "node", "react", "std", "tooling"], root);
    const commits = !made
      ? `The pin's commit says no \`Regenerate on rust-js <sha>\`, so which of rust-js's commits made these isn't known.`
      : since && since.ok
        ? `rust-js's commits since ${made}, which the pin's JS was made on, that change the compiler, its bindings or its runtime: each change above is one of theirs.\n\n${since.stdout.split("\n").filter(Boolean).map((line) => line.slice(0, 220)).slice(0, 200).join("\n")}`
        : `rust-js's history here doesn't reach ${made}, which the pin's JS was made on: fetch it (\`fetch-depth: 0\`) to list the commits since.`;
    report(
      `### react-in-rust at ${pin.slice(0, 9)}: ${changed.length} file(s) differ from what it committed\n\n` +
        `${changed.map((line) => `- \`${line}\``).join("\n")}\n\n<details><summary>The first lines of each</summary>\n\n\`\`\`diff\n${shown}\n\`\`\`\n</details>\n\n` +
        `<details><summary>What may have made them</summary>\n\n${commits}\n</details>\n\n` +
        "A change one of those commits intends (its ADR or `rd` gap says so) waits for the port's next regeneration, which moves `ports/react-in-rust.pin`; any other is a regression here, fixed or filed.",
    );
  }
  if (failed.length || changed.length) process.exit(1);
  report(`react-in-rust at ${pin.slice(0, 9)}: every crate compiles, and its JS is what it committed.`);
} else {
  console.error("usage: bun scripts/react-in-rust.ts check | regen [crate ..]");
  process.exit(2);
}
