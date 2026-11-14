// Mutations of src/library/scopes.rs, what makes each fact of a library's contract true (ADR 1129, amended; #1671 change 4 M4).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "contract-closed-world-fact-a-summary",
    breaks:
      "`ParamFact::Unboxed`, true only of what its crate's every call gives, is said to be a summary of its function, with no word of the consumer that gives otherwise, which must compile its own (ADR 0051)",
    file: "src/library/scopes.rs",
    find: '                "ParamFact::Unboxed",\n                ClosedWorld(CompileOwn("compiles its own of the function (ADR 0051)")),\n',
    replace: '                "ParamFact::Unboxed",\n                Summary,\n',
    unitTests: "library::scopes::tests::contract_table",
  },
  {
    name: "contract-fact-listed-twice",
    breaks: "a field of the contract is given another's `ContractField`, so one fact has two rows and the other none",
    file: "src/library/scopes.rs",
    find: "    keeps_capacity => LibraryKeepsCapacity,\n",
    replace: "    keeps_capacity => LibraryWritesMayFail,\n",
    unitTests: "library::scopes::tests::each_fact_says_what_makes_it_true",
  },
];
