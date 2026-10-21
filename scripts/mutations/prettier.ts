// Mutations of src/prettier.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "prettier-never-asked",
    breaks: "the app's Prettier is never run, and an app with a config has rust-js's layout, which its format check fails",
    file: "src/prettier.rs",
    find: "    if files.is_empty() || settings.dir.as_os_str().is_empty() || !settings.cargo || cfg!(target_os = \"wasi\") {",
    replace: "    if true {",
    tests: ["test/settings.test.ts", "-t", "where Prettier finds a config"],
  },
  {
    name: "prettier-without-a-config",
    breaks: "a file Prettier has no config for is laid out by Prettier's defaults, and a crate without one isn't rust-js's layout",
    file: "src/prettier.rs",
    find: "  if (!config) {",
    replace: "  if (false) {",
    tests: ["test/settings.test.ts", "-t", "no config of its own keeps rust-js's layout"],
  },
  {
    name: "prettier-for-a-file-compiled-alone",
    breaks: "a file compiled alone, a test's, is laid out by whatever Prettier config is above it, and each compile starts Node to ask",
    file: "src/prettier.rs",
    find: "settings.dir.as_os_str().is_empty() || !settings.cargo ||",
    replace: "settings.dir.as_os_str().is_empty() ||",
    tests: ["test/settings.test.ts", "-t", "a file compiled alone keeps"],
  },
  {
    name: "prettier-editorconfig-ignored",
    breaks: "Prettier's config is read without the app's `.editorconfig`, which its own `prettier --check` reads",
    file: "src/prettier.rs",
    find: "{ editorconfig: true }",
    replace: "{ editorconfig: false }",
    tests: ["test/settings.test.ts", "-t", "editorconfig"],
  },
  {
    name: "prettier-commonjs-unread",
    breaks: "Prettier 2's API, CommonJS, its module's `default`, is looked for by name, and an app on Prettier 2, react.dev, fails its build",
    file: "src/prettier.rs",
    find: "  prettier = typeof module.format === \"function\" ? module : module.default;",
    replace: "  prettier = module;",
    tests: ["test/settings.test.ts", "-t", "CommonJS Prettier"],
  },
];
