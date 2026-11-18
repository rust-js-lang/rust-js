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
  {
    name: "regexp-test-counted-as-running-code",
    breaks: "a regex's `test` loses `runs_nothing`, so a memo whose maker only tests a regex keeps `$withBorrowMut` around its cache (#2082)",
    file: "builtins/src/lib.rs",
    find: '#[cfg_attr(rust_js, rust_js::link_name = "test")]\n    #[cfg_attr(rust_js, rust_js::runs_nothing)]\n',
    replace: '#[cfg_attr(rust_js, rust_js::link_name = "test")]\n',
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "runs only a regex"],
  },
  {
    name: "regexp-exec-not-creating",
    breaks: "a regex's `exec` loses `creates`, so a property read of its match, `matched.get(1)`, keeps a memo's borrow checked (#2106)",
    file: "builtins/src/lib.rs",
    find: '#[cfg_attr(rust_js, rust_js::link_name = "exec")]\n    #[cfg_attr(rust_js, rust_js::creates)]\n',
    replace: '#[cfg_attr(rust_js, rust_js::link_name = "exec")]\n',
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "the match exec made"],
  },
];
