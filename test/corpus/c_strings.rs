// A C string is its bytes with its nul (#1157, the user's decision): a
// `CStr` as a `[u8]` is, a JS array of numbers, `c"hi"` `[104, 105, 0]`; a
// `CString` its `Vec<u8>` with the nul. Each constructor and accessor, its
// errors' `Debug` and `Display`, `{:?}`'s escapes of text that isn't ASCII
// or UTF-8, and what std derives of them, `==`, ordering, `Clone`, as
// a set's and a map's keys.
// They were refused.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::{CStr, CString};

fn main() {
    // Literals, their bytes and their Debug's escapes.
    let odd = c"tab\there \"q\" \\ é \x7f \xff end";
    println!("{odd:?} {} {}", odd.count_bytes(), odd.is_empty());
    println!("{:?} {}", c"".to_bytes(), c"".is_empty());
    println!("{:?} {:?}", odd.to_str().is_err(), odd.to_string_lossy());
    println!("{:?}", c"ok".to_str());
    println!("{:?}", odd.to_bytes_with_nul().last());

    // Made of bytes, and refused.
    println!("{:?}", CStr::from_bytes_with_nul(b"ab\0"));
    println!("{:?}", CStr::from_bytes_with_nul(b"a\0b\0"));
    println!("{:?}", CStr::from_bytes_with_nul(b"ab"));
    println!("{:?}", CStr::from_bytes_until_nul(b"ab\0cd\0"));
    println!("{:?}", CStr::from_bytes_until_nul(b"abc"));
    if let Err(e) = CStr::from_bytes_with_nul(b"a\0b\0") {
        println!("{e} | {e:#?}");
    }
    if let Err(e) = CStr::from_bytes_with_nul(b"ab") {
        println!("{e}");
    }
    if let Err(e) = CStr::from_bytes_until_nul(b"x") {
        println!("{e} | {e:#?}");
    }

    // Owned, of each kind of bytes.
    let a = CString::new("text").unwrap();
    let b = CString::new(String::from("owned")).unwrap();
    let c = CString::new(vec![65u8, 66]).unwrap();
    let d = CString::new(&b"slice"[..]).unwrap();
    let e = CString::new([0x41u8, 0x42]).unwrap();
    println!("{a:?} {b:?} {c:?} {d:?} {e:?}");
    let bad = CString::new("a\0b").unwrap_err();
    println!("{bad:#?}");
    println!("{bad} | {bad:?} | {}", bad.nul_position());
    println!("{:?}", bad.into_vec());

    println!("{:?} {:?} {:?}", a.as_bytes(), a.as_bytes_with_nul(), a.as_c_str());
    println!("{:?} {}", a.to_str(), a.count_bytes());
    let bytes: Vec<u8> = b.clone().into_bytes();
    let with_nul = c.clone().into_bytes_with_nul();
    println!("{bytes:?} {with_nul:?}");
    println!("{:?}", d.clone().into_string());
    let invalid = unsafe { CString::from_vec_unchecked(vec![0xffu8, 0x41]) };
    let failed = invalid.clone().into_string().unwrap_err();
    println!("{invalid:?} {failed} | {failed:?}");
    println!("{:?} {:?}", failed.utf8_error(), failed.into_cstring());

    // Compared, ordered, hashed, cloned and converted.
    let owned: CString = c"hi".to_owned();
    let from: CString = CString::from(c"hi");
    println!("{} {} {}", owned == from, owned.as_c_str() == c"hi", c"a" < c"ab");
    let mut sorted = vec![c"b", c"ab", c"a", c""];
    sorted.sort();
    println!("{sorted:?}");
    let set: HashSet<&CStr> = [c"x", c"y", c"x"].into_iter().collect();
    println!("{}", set.len());
    let mut map: HashMap<CString, u32> = HashMap::new();
    map.insert(CString::new("k").unwrap(), 7);
    println!("{:?}", map.get(c"k"));
    let v: Vec<u8> = Vec::from(owned.clone());
    println!("{v:?} {:?} {:?}", CString::default(), <&CStr>::default());
    let r: &CStr = owned.as_ref();
    println!("{:?} {:?}", r, Some(c"some"));

    // Held in a B-tree, in order.
    let tree: BTreeSet<CString> = ["b", "a", "ab"].map(|s| CString::new(s).unwrap()).into();
    println!("{tree:?}");
    let all: Vec<CString> = vec![CString::new("x").unwrap(), CString::default()];
    println!("{all:?} {all:#?}");
    println!("{:?}", c"e\u{301} \u{1F600} \n");
    let pretty = CString::from_vec_with_nul(vec![0xc3, 0]).unwrap().into_string().unwrap_err();
    println!("{pretty:#?}");
    let kept = CString::from_vec_with_nul(b"ok\0".to_vec()).unwrap();
    let interior = CString::from_vec_with_nul(b"a\0b\0".to_vec()).unwrap_err();
    let unended = CString::from_vec_with_nul(b"ab".to_vec()).unwrap_err();
    println!("{kept:?} {interior} | {interior:?} | {unended} | {unended:?}");
    println!("{interior:#?} {:?}", unended.as_bytes());

    // Ordered by their bytes, unsigned, a byte past 0x7f after ASCII.
    let mut by_bytes: BTreeMap<&CStr, usize> = BTreeMap::new();
    for (i, k) in [c"b", c"\xff", c"a", c"ab", c"", c"\x7f"].into_iter().enumerate() {
        by_bytes.insert(k, i);
    }
    println!("{by_bytes:?} {:?}", by_bytes.get(c"ab"));
    let mut high = vec![CString::new(vec![0x80u8]).unwrap(), CString::new("z").unwrap()];
    high.sort();
    println!("{high:?} {:?} {}", c"\xff".cmp(c"a"), c"\xff" > c"a");
}
