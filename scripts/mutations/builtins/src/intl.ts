// Mutations of builtins/src/intl.rs (ADR 0093), the generated Intl
// bindings: a mark builtins/policy.ts writes, unread (#2116).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "week-info-not-creating",
    breaks: "`getWeekInfo` isn't marked `creates`, so the week info a made locale's `getWeekInfo?.()` makes is a JS value read under a held borrow, counted (#2116)",
    file: "builtins/src/intl.rs",
    find: "    #[cfg_attr(rust_js, rust_js::link_name = \"getWeekInfo\")]\n    #[cfg_attr(rust_js, rust_js::runs_nothing)]\n    #[cfg_attr(rust_js, rust_js::creates)]\n",
    replace: "    #[cfg_attr(rust_js, rust_js::link_name = \"getWeekInfo\")]\n    #[cfg_attr(rust_js, rust_js::runs_nothing)]\n",
    tests: ["test/at-hand-returns.test.ts", "-t", "a made locale's getWeekInfo"],
  },
];
