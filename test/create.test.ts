// `bun create @rust-js my-app` (ADR 0105): @rust-js/create makes a Vite and
// React app written in Rust, the vite-react example's files, named for its
// directory, with rust-js's packages at the release's version.

import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { runSync } from "./child";
import { fixture, root, run } from "./support";
import { channel } from "../scripts/toolchain.ts";
import { writePatch } from "../tooling/patch.js";

const pkg = JSON.parse(readFileSync(join(root, "create", "package.json"), "utf8"));
const version = readFileSync(join(root, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];

/** @rust-js/create as a registry would give it: packed, then unpacked. */
function packed(): string {
  const dir = fixture("create-packed");
  run([process.execPath, "scripts/package-create.ts", join(dir, "create.tgz")]);
  run(["tar", "-xzf", join(dir, "create.tgz"), "-C", dir]);
  return join(dir, "package", "index.js");
}

/** Every file under `dir`, relative to it, sorted. */
function files(dir: string): string[] {
  const out: string[] = [];
  const walk = (d: string) => {
    for (const name of readdirSync(d)) {
      const path = join(d, name);
      if (statSync(path).isDirectory()) walk(path);
      else out.push(relative(dir, path));
    }
  };
  walk(dir);
  return out.sort();
}

// Released with the compiler, so the app it makes names that release's
// packages, and the Rust it says to install is the compiler's.
test("@rust-js/create is the compiler's version, and says its Rust channel", () => {
  expect([pkg.name, pkg.version, pkg.rustJs.toolchain]).toEqual(["@rust-js/create", version, channel]);
});

test("a packed @rust-js/create makes the vite-react example's app, named for its directory", () => {
  const index = packed();
  const cwd = fixture("create-app");
  const made = runSync([process.execPath, index, "my-app"], cwd, 60_000);
  expect(made.code).toBe(0);
  const app = join(cwd, "my-app");
  // The example's files, as git has them: not what building it made.
  const example = run(["git", "ls-files", "examples/vite-react"]).trim().split("\n")
    .map((file) => relative("examples/vite-react", file)).sort();
  expect(files(app)).toEqual(example);
  // npm leaves a package's `.gitignore` out; the app has its own.
  expect(readFileSync(join(app, ".gitignore"), "utf8")).toBe(readFileSync(join(root, "examples/vite-react/.gitignore"), "utf8"));
  // The app's README is the app's, not this repository's example's.
  expect(readFileSync(join(app, "README.md"), "utf8")).not.toContain("repository root");
  const manifest = JSON.parse(readFileSync(join(app, "package.json"), "utf8"));
  expect(manifest.name).toBe("my-app");
  // Its editor checks it through rust-js, which reads inside JSX (ADR 0222).
  expect(readFileSync(join(app, ".vscode/settings.json"), "utf8")).toContain('"rust-analyzer.check.overrideCommand": ["./node_modules/.bin/rust-js-check"]');
  expect(manifest.devDependencies["@rust-js/build"]).toBe(version);
  expect(manifest.dependencies["@rust-js/runtime"]).toBe(version);
  expect(manifest.scripts.build).toBe("rust-js build --check && vite build");
  // The compiler, which a release of @rust-js/build doesn't bring: the app's own.
  expect(manifest.devDependencies["@rust-js/native"]).toBe(version);
  expect(JSON.stringify(manifest)).not.toContain("workspace:");
  // What to run next, the Rust it needs first.
  expect(made.stdout).toContain(`rustup toolchain install ${channel} --profile minimal --target wasm32-unknown-unknown
`);
  expect(made.stdout).toContain("cd my-app");
  expect(made.stdout).toContain("bun install");
});

// `--template next`: the Next.js example's app (ADR 0192), its page Rust,
// its crates npm packages as the Vite one's are, and `next` among them.
test("--template next makes the Next.js example's app, named for its directory", () => {
  const index = packed();
  const cwd = fixture("create-next");
  const made = runSync([process.execPath, index, "my-site", "--template", "next"], cwd, 60_000);
  expect(made.code).toBe(0);
  const app = join(cwd, "my-site");
  const example = run(["git", "ls-files", "examples/next"]).trim().split("\n")
    .map((file) => relative("examples/next", file)).sort();
  expect(files(app)).toEqual(example);
  expect(readFileSync(join(app, "app/page.rs"), "utf8")).toBe(readFileSync(join(root, "examples/next/app/page.rs"), "utf8"));
  const manifest = JSON.parse(readFileSync(join(app, "package.json"), "utf8"));
  const crate = (dir: string) => readFileSync(join(root, dir, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];
  expect(manifest.name).toBe("my-site");
  expect(manifest.scripts).toEqual({ dev: "next dev", build: "rust-js build --check && next build", start: "next start", lint: "eslint" });
  expect(manifest.dependencies).toMatchObject({ "@rust-js/next": crate("next"), "@rust-js/react": crate("react"), "@rust-js/webapi": crate("webapi"), "@rust-js/builtins": crate("builtins") });
  expect(readFileSync(join(app, ".vscode/settings.json"), "utf8")).toContain("rust-js-check");
  expect(manifest.devDependencies["@rust-js/build"]).toBe(version);
  expect(JSON.stringify(manifest)).not.toContain("workspace:");
  const cargoToml = readFileSync(join(app, "Cargo.toml"), "utf8");
  expect(cargoToml).not.toContain('path = "../');
  expect(cargoToml).toContain(`next = { package = "rust-js-next", version = "~${crate("next")}" }`);
  expect(made.stdout).toContain("Made my-site: a Next.js app, its page app/page.rs, in Rust.");
  expect(runSync([process.execPath, index, "other", "--template", "remix"], cwd, 60_000).stderr).toContain("--template is vite or next");
});

// An app is a Cargo package, which Vite builds in Cargo's way (ADR 0101),
// and an editor checks: its crates, `js`, `webapi` and `react`, are npm
// packages, named in its Cargo.toml by version, which the patch `rust-js
// build` writes, and the editor's check, tells Cargo are in node_modules.
test("a created app has its crates from npm, which its patch tells Cargo where they are", () => {
  const index = packed();
  const cwd = fixture("create-cargo");
  expect(runSync([process.execPath, index, "app"], cwd, 60_000).code).toBe(0);
  const app = join(cwd, "app");
  const manifest = JSON.parse(readFileSync(join(app, "package.json"), "utf8"));
  const crate = (dir: string) => readFileSync(join(root, dir, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];
  expect(manifest.dependencies).toMatchObject({ "@rust-js/builtins": crate("builtins"), "@rust-js/webapi": crate("webapi"), "@rust-js/react": crate("react") });
  expect(manifest.devDependencies["@rust-js/build"]).toBe(version);
  expect(manifest.scripts.postinstall).toBeUndefined();
  const cargoToml = readFileSync(join(app, "Cargo.toml"), "utf8");
  expect(cargoToml).not.toContain('path = "../');
  expect(cargoToml).toContain(`react = { package = "rust-js-react", version = "~${crate("react")}" }`);
  // As a package manager installs them: the packed packages, unpacked.
  const packs = fixture("create-crates");
  run([process.execPath, "scripts/package-npm-crates.ts", packs], 600_000);
  for (const name of ["builtins", "webapi", "react"]) {
    const at = join(app, "node_modules", "@rust-js", name);
    mkdirSync(at, { recursive: true });
    run(["tar", "-xzf", join(packs, `${name}.tgz`), "-C", at, "--strip-components", "1"]);
  }
  writePatch(app);
  const check = runSync(["cargo", "check", "--offline", "--quiet", "--manifest-path", join(app, "Cargo.toml")], app, 300_000, { RUSTC_BOOTSTRAP: undefined });
  // No error, and no warning of the crates it uses: the app's are of what's
  // used only in JSX, which react's placeholder `jsx!` doesn't look inside,
  // and of its component's capitalized name, which rust-js's own pass allows
  // and a plain rustc doesn't run (ADR 0439).
  const warnings = check.stderr.split("\n").filter((line) => /^(warning|error)\b/.test(line) && !/\(lib\) generated \d+ warnings?/.test(line));
  expect(warnings.filter((line) => !/^warning: (unused variable: |static `\w+` is never used|function `App` should have a snake case name)/.test(line))).toEqual([]);
  expect(check.stderr).not.toMatch(/`rust-js-\w+` \(lib\) generated/);
  expect(check.code).toBe(0);

  // A Next.js app's, `next` among them, as the same packages.
  expect(runSync([process.execPath, index, "site", "--template", "next"], cwd, 60_000).code).toBe(0);
  const site = join(cwd, "site");
  for (const name of ["builtins", "webapi", "react", "next", "node"]) {
    const at = join(site, "node_modules", "@rust-js", name);
    mkdirSync(at, { recursive: true });
    run(["tar", "-xzf", join(packs, `${name}.tgz`), "-C", at, "--strip-components", "1"]);
  }
  writePatch(site);
  const next = runSync(["cargo", "check", "--offline", "--quiet", "--manifest-path", join(site, "Cargo.toml")], site, 300_000, { RUSTC_BOOTSTRAP: undefined });
  // What's used only in JSX, `Image`, which react's placeholder `jsx!` doesn't look inside,
  // and the page's component's capitalized name, which only rust-js's pass allows (ADR 0439).
  const nextWarnings = next.stderr.split("\n").filter((line) => /^(warning|error)\b/.test(line) && !/\(lib\) generated \d+ warnings?/.test(line));
  expect(nextWarnings.filter((line) => !/^warning: (unused import: |function `Home` should have a snake case name)/.test(line))).toEqual([]);
  expect(next.code).toBe(0);
}, 900_000);

// Before a release is on npm: the packages `pack:distribution` made, each
// the app's, and `@rust-js/build` the plugin's too, as Qualify installs them.
// An app is Bun's or Node's: what to run next is the package manager's
// that ran it, as it says in `npm_config_user_agent`, or else the runtime's.
test("@rust-js/create says the commands of the package manager that ran it", () => {
  const index = packed();
  const next = (agent: string | undefined, runtime = process.execPath) => {
    const cwd = fixture("create-agent");
    const made = runSync([runtime, index, "app"], cwd, 60_000, { npm_config_user_agent: agent });
    expect(made.code).toBe(0);
    return made.stdout.slice(made.stdout.indexOf("cd app")).trimEnd();
  };
  // And, in another terminal, the app's Rust compiled (ADR 0412).
  const then = (manager: string, runner: string) => `cd app\n  ${manager} install\n  ${manager} run dev\n\nAnd in another terminal, after you change its Rust:\n\n  ${runner} rust-js build`;
  expect(next("npm/10.9.2 node/v22.12.0 darwin arm64 workspaces/false")).toBe(then("npm", "npx"));
  expect(next("bun/1.4.2 npm/? node/v24.3.0 darwin arm64")).toBe(then("bun", "bunx"));
  expect(next("pnpm/10.12.1 npm/? node/v22.12.0 linux x64")).toBe(then("pnpm", "npx"));
  // Run by Node itself, `node index.js app`: npm's.
  expect(next(undefined, "node")).toBe(then("npm", "npx"));
});

// What its README says to install is what it prints, at this channel.
test("@rust-js/create's README says to install the Rust it prints", () => {
  const readme = readFileSync(join(root, "create", "README.md"), "utf8");
  expect(readme).toContain(`rustup toolchain install ${channel} --profile minimal --target wasm32-unknown-unknown`);
});

test("--local makes the app install a distribution's packages", () => {
  const index = packed();
  const dist = fixture("create-dist");
  const artifacts = [["@rust-js/runtime", "runtime.tgz"], ["@rust-js/build", "build.tgz"], ["@rust-js/typescript", "typescript.tgz"], ["@rust-js/native", "native.tgz"]]
    .map(([name, file]) => ({ file, package: name }));
  writeFileSync(join(dist, "distribution.json"), JSON.stringify({ version: 1, platform: process.platform, arch: process.arch, compiler: { version }, artifacts }));
  const cwd = fixture("create-local");
  const made = runSync([process.execPath, index, "app", "--local", dist], cwd, 60_000);
  expect(made.code).toBe(0);
  const manifest = JSON.parse(readFileSync(join(cwd, "app", "package.json"), "utf8"));
  const file = (name: string) => `file:${join(dist, name)}`;
  expect(manifest.dependencies["@rust-js/runtime"]).toBe(file("runtime.tgz"));
  expect(manifest.devDependencies).toMatchObject({
    "@rust-js/build": file("build.tgz"),
    "@rust-js/native": file("native.tgz"),
  });
  expect(manifest.overrides).toEqual({ "@rust-js/build": file("build.tgz"), "@rust-js/typescript": file("typescript.tgz") });
});

test("--local refuses a directory that isn't a distribution", () => {
  const index = packed();
  const cwd = fixture("create-no-dist");
  const made = runSync([process.execPath, index, "app", "--local", cwd], cwd, 60_000);
  expect(made.code).not.toBe(0);
  expect(made.stderr).toContain("distribution.json");
  expect(existsSync(join(cwd, "app"))).toBe(false);
});

// It never writes over what's there.
test("@rust-js/create refuses a directory that isn't empty", () => {
  const index = packed();
  const cwd = fixture("create-taken");
  mkdirSync(join(cwd, "taken"));
  writeFileSync(join(cwd, "taken", "notes.txt"), "mine\n");
  const made = runSync([process.execPath, index, "taken"], cwd, 60_000);
  expect(made.code).not.toBe(0);
  expect(made.stderr).toContain("isn't empty");
  expect(readdirSync(join(cwd, "taken"))).toEqual(["notes.txt"]);
});

test("@rust-js/create without a directory says how to use it", () => {
  const index = packed();
  const made = runSync([process.execPath, index], fixture("create-usage"), 60_000);
  expect(made.code).not.toBe(0);
  expect(made.stderr).toContain("bun create @rust-js@latest <directory>");
});
