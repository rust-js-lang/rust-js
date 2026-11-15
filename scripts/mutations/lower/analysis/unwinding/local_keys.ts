// Mutations of the LocalKey access summary (#1833).
import type {Mutation} from "../../../../mutations";
export const mutations: Mutation[] = [
  {
    name: "local-with-lazy-initializer-unseen",
    breaks: "a lazy initializer that panics is omitted from a destructor's effects",
    file: "src/lower/analysis/unwinding/local_keys.rs",
    find: "        if !key.as_local().is_some_and(|key| eager.contains(&key)) {\n",
    replace: "        if false && !key.as_local().is_some_and(|key| eager.contains(&key)) {\n",
    tests: ["test/lowering-local-key-unwinding.test.ts", "-t", "lazy LocalKey"],
  },
  {
    name: "local-with-eager-key-unproved",
    breaks: "an eager LocalKey is rejected even when its callback cannot unwind",
    file: "src/lower/analysis/unwinding/local_keys.rs",
    find: "        if !key.as_local().is_some_and(|key| eager.contains(&key)) {\n",
    replace: "        if true || !key.as_local().is_some_and(|key| eager.contains(&key)) {\n",
    tests: ["test/lowering-local-key-unwinding.test.ts", "-t", "proved nonthrowing"],
  },
  {
    name: "local-with-unrelated-callee-trusted",
    breaks: "an unrelated throwing function with key/callback arguments is treated as LocalKey with",
    file: "src/lower/analysis/unwinding/local_keys.rs",
    find: "        if !matches!(recognition.classify(callee, types), Some(Std::LocalWith { .. })) {\n",
    replace: "        if false && !matches!(recognition.classify(callee, types), Some(Std::LocalWith { .. })) {\n",
    tests: ["test/lowering-local-key-unwinding.test.ts", "-t", "unrelated function"],
  },
];
