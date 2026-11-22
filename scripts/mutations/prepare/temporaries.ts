// Mutations of crates/rust-js-program/src/prepare/temporaries.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "temporary-takes-a-word-reclaimed",
    breaks:
      "a temporary takes `next` beside the `next$7` lowering spelled, which reclaiming would make `next`: the program's names outrank a made one (ADR 0357, #1681)",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "                                name.apart_of().is_some_and(|(of, _)| of == to)\n",
    replace: "                                false\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-takes-an-import-reclaimed",
    breaks:
      "a temporary takes `createContainer`, which the import given apart at the module's top, `createContainer$1`, would be reclaimed to, so the import stays apart (ADR 0569)",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "                    }) && !reclaimed.contains(to)\n",
    replace: "                    })\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_beside_an_import_of_its_name.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-flags-of-the-old-word",
    breaks:
      "a spilled value's drop flag keeps the word it was made of, `value$0$live` beside `tryLock`, not `tryLock$0$live` (#841)",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "                flags.insert(name.clone(), name.suffixed_again(source));\n",
    replace: "                let _ = source;\n",
    tests: ["test/corpus.test.ts", "-t", "^lock_poisoning.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-name-taken-suffixed",
    breaks:
      "a temporary whose suggested names are taken is given one apart, `point$1`, not `value`",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "                    tree.as_ref().is_some_and(|tree| {\n",
    replace: "                    true || tree.as_ref().is_some_and(|tree| {\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-takes-a-name-items-read",
    breaks:
      "a temporary beside a function written in the body takes the name of the module's function that function reads, `note`, which preparation then gives apart, `const note$1 = id(k)`, not its own `value`, where it once threw `note is not a function` (ADR 1150)",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "                            && tree.renames(from, to)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^temporaries_beside_items_written_in_the_body.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-takes-import-name",
    breaks:
      "a temporary is named after a function another module gives this one, so its import is renamed `prepare$1`",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "        let mut tree = scopes::Tree::of_in(item, &top);\n",
    replace: "        let mut tree = scopes::Tree::of_in(item, &[]);\n",
    tests: ["test/declarations-libraries.test.ts", "-t", "a library without declarations has its types as any"],
    snapshots: true,
  },
  {
    name: "temporaries-named-by-nesting",
    breaks:
      "temporaries are named outer lists first, so a `let` set late takes `next` from one read first",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "        suggested.sort_by_key(|name| first.get(name).copied().unwrap_or(usize::MAX));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(loop_drops|assignment_order).rs$"],
    snapshots: true,
  },
  {
    name: "temporary-declared-again-kept",
    breaks: "two copies of a `Drop`'s body applied beside each other in one block keep one made name, which preparation spells `value` for both, `const value` declared twice, so rustc's drop/issue-48962.rs and drop/struct-field-drop-order.rs are refused (#1681 change 3, step 2's regression)",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "        apart_where_declared_again(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(drop_body_inlined_twice_in_a_block|field_drops_inlined_in_one_block)\\.rs$"],
  },
  {
    name: "temporary-takes-the-programs-unspelled-word",
    breaks: "a temporary takes the word a name of the program's not spelled yet, a scope's function, will be spelled, which then pushes it apart (#2048)",
    file: "crates/rust-js-program/src/prepare/temporaries.rs",
    find: "                                    || (name.is_the_programs() && name.made_base() == Some(to))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(format_args_order|scope_struct_functions|mut_ref_held_written_whole)\\.rs$"],
    snapshots: true,
  },
];
