import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "string-replace-all-is-first-only",
    breaks: "a native replacement of every match calls replace and stops after the first match",
    file: "builtins/src/lib.rs",
    find: '#[cfg_attr(rust_js, rust_js::link_name = "replaceAll")]\n    pub fn replace_all',
    replace: '#[cfg_attr(rust_js, rust_js::link_name = "replace")]\n    pub fn replace_all',
    tests: ["test/js-strings.test.ts", "-t", "replaceAll uses JavaScript replacement templates"],
  },
];
