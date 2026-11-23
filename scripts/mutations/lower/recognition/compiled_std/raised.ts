// Mutations of crates/rust-js-lower/src/recognition/compiled_std/raised.rs (ADR 1184).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "raised-call-at-its-whole-span",
    breaks: "a std call's site is where its expression starts, not its method's name, as codegen's `fn_span`",
    file: "crates/rust-js-lower/src/recognition/compiled_std/raised.rs",
    find: "                            span: *fn_span,\n",
    replace: "                            span: terminator.source_info.span,\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "raised-inlined-bodies-own",
    breaks: "a check inside a body inlined of another function counts as the walked function's, `is_char_boundary`'s in `split_at_mut`",
    file: "crates/rust-js-lower/src/recognition/compiled_std/raised.rs",
    find: "                (false, false) => return None,\n",
    replace: "                (false, false) => break,\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "raised-nested-unfollowed",
    breaks: "the walk doesn't follow a function's nested items, so `swap_remove`'s site, its cold `assert_failed`'s, isn't found",
    file: "crates/rust-js-lower/src/recognition/compiled_std/raised.rs",
    find: "                    } else if nested_in(tcx, callee, root) {\n",
    replace: "                    } else if false {\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "raised-track-caller-scopes-unwalked",
    breaks: "a site inside a body inlined of a function that tracks its caller names that body's line, not where it's called, as codegen does",
    file: "crates/rust-js-lower/src/recognition/compiled_std/raised.rs",
    find: "                (true, _) => span = call,\n",
    replace: "                (true, _) => break,\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "given-closures-unwalked",
    breaks: "the walk of std's MIR leaves out the closures a function gives what it calls, so `with_borrow`'s borrow, in its closure `with` calls, and `set`'s, given `initialize_with`, aren't sites (#2292)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/raised.rs",
    find: "                            each.push(*closure);\n",
    replace: "",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "bounded-impl-blanket-unmatched",
    breaks: "a blanket impl std specializes by its bound isn't found, so `ToRcSlice`'s `capacity overflow` isn't a site (#2292)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/raised.rs",
    find: "                None => tcx.trait_impls_of(trait_).blanket_impls().to_vec(),\n",
    replace: "                None => Vec::new(),\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "vendored-site-named-locally",
    breaks: "a site in a crate std vendors, hashbrown's, Cargo's build took from the registry, is named by this machine's registry path, not the vendored crate's, so the runtime's sites name a machine's own source (#2325)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/raised.rs",
    find: "    let file = match file.find(\"/registry/src/\") {\n",
    replace: "    let file = match None::<usize> {\n",
    tests: ["test/std-sites.test.ts", "-t", "the runtime's sites of std's panics are what std's MIR says"],
  },
];
