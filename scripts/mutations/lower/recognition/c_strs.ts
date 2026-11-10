// Mutations of src/lower/recognition/c_strs.rs (ADR 1119).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "c-str-until-nul-as-with-nul",
    breaks: "`CStr::from_bytes_until_nul(b\"ab\\0cd\\0\")` is refused as `from_bytes_with_nul`'s interior nul",
    file: "src/lower/recognition/c_strs.rs",
    find: "\"std::ffi::CStr::from_bytes_until_nul\" => CStrOp::UntilNul,",
    replace: "\"std::ffi::CStr::from_bytes_until_nul\" => CStrOp::WithNul,",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-str-nul-error-as-into-string",
    breaks: "a `NulError` is shown as an `IntoStringError`",
    file: "src/lower/recognition/c_strs.rs",
    find: "\"NulError\" if alloc => CStrError::Nul,",
    replace: "\"NulError\" if alloc => CStrError::IntoString,",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-string-from-c-str-unknown",
    breaks: "`CString::from(c)` of a `&CStr` is refused",
    file: "src/lower/recognition/c_strs.rs",
    find: "        return Some(Std::CStr(CStrOp::Owned));\n",
    replace: "        return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-string-into-vec-kept-whole",
    breaks: "`Vec::from(s)` of a `CString` keeps its nul",
    file: "src/lower/recognition/c_strs.rs",
    find: "(vec && is_c_string(tcx, from)).then_some(Std::CStr(CStrOp::IntoBytes))",
    replace: "(vec && is_c_string(tcx, from)).then_some(Std::Same)",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-string-error-kind-not-known",
    breaks:
      "a clone or `==` of a `FromVecWithNulError` is refused, the kind it holds not known as a C string's error's part (#1157)",
    file: "src/lower/recognition/c_strs.rs",
    find: "                && tcx.item_name(adt.did()).as_str() == \"FromBytesWithNulErrorKind\")\n",
    replace: "                && tcx.item_name(adt.did()).as_str() == \"FromBytesWithNulError\")\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings_hashed_and_their_errors_cloned.rs$"],
  },
];
