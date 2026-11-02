// `*r = v` of a `&mut` read from where one's kept, a `Result`'s or an
// `Option`'s payload, a tuple's, a `Vec`'s or a struct's field: what it
// refers to becomes `v`, the map's entry, the variable; not the slot that
// holds the reference, `result._0 = v`, which left the entry as it was
// (#466). A `Vec`'s item written whole is still its place.

use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy)]
struct Point {
    x: i32,
}

#[derive(Debug)]
struct Account {
    balance: u32,
}

struct Holder<'a> {
    account: &'a mut Account,
}

struct Outer<'a> {
    inner: Holder<'a>,
}

struct Slot<'a> {
    current: Option<&'a mut Account>,
}

struct PointHolder<'a> {
    point: &'a mut Point,
}

struct Pair {
    a: Point,
}

struct Ledger {
    by_name: HashMap<String, Account>,
}

impl Ledger {
    fn named(&mut self, name: &str) -> Option<&mut Account> {
        self.by_name.get_mut(name)
    }
}

fn tree() -> Result<u32, String> {
    let mut tree = BTreeMap::from([("demo".to_string(), Account { balance: 0 })]);
    *tree.get_mut("demo").ok_or_else(|| "missing".to_string())? = Account { balance: 1 };
    Ok(tree["demo"].balance)
}

fn hash() -> Result<u32, String> {
    let mut hash = HashMap::from([("demo", Account { balance: 0 })]);
    *hash.get_mut("demo").ok_or_else(|| "missing".to_string())? = Account { balance: 2 };
    Ok(hash["demo"].balance)
}

fn option() -> Result<u32, String> {
    let mut option = Some(Account { balance: 0 });
    *option.as_mut().ok_or_else(|| "missing".to_string())? = Account { balance: 3 };
    Ok(option.unwrap().balance)
}

fn holder() -> u32 {
    let mut account = Account { balance: 0 };
    let holder = Holder { account: &mut account };
    *holder.account = Account { balance: 4 };
    account.balance
}

fn vec_item() -> u32 {
    let mut values = vec![Account { balance: 0 }];
    values.push(Account { balance: 0 });
    values[0] = Account { balance: 5 };
    values[0].balance + values[1].balance
}

fn tuple() -> u32 {
    let mut a = Account { balance: 0 };
    let t = (&mut a, 1);
    *t.0 = Account { balance: 6 };
    a.balance
}

fn vec_of_refs() -> u32 {
    let mut a = Account { balance: 0 };
    let mut b = Account { balance: 0 };
    let mut refs = vec![&mut a];
    refs.push(&mut b);
    *refs[0] = Account { balance: 7 };
    a.balance + b.balance
}

fn nested() -> u32 {
    let mut a = Account { balance: 0 };
    let outer = Outer {
        inner: Holder { account: &mut a },
    };
    *outer.inner.account = Account { balance: 8 };
    a.balance
}

fn copy_first() -> Result<String, String> {
    let p = Point { x: 0 };
    let mut v = vec![p];
    v.push(p);
    *v.first_mut().ok_or("empty")? = Point { x: 9 };
    Ok(format!("{} {p:?} {v:?}", p.x))
}

fn copy_in_map() -> Result<String, String> {
    let p = Point { x: 0 };
    let mut map = HashMap::from([(1, p)]);
    let q = map[&1];
    *map.get_mut(&1).ok_or("missing")? = Point { x: 10 };
    Ok(format!("{p:?} {q:?} {:?}", map[&1]))
}

fn holder_moved_on() -> u32 {
    let mut first = Account { balance: 0 };
    let mut other = Account { balance: 100 };
    let mut h = Holder { account: &mut first };
    let r: &mut Account = &mut *h.account;
    *r = Account { balance: 11 };
    h.account = &mut other;
    *h.account = Account { balance: 12 };
    first.balance * 1000 + other.balance
}

fn slot_field() -> u32 {
    let mut a = Account { balance: 0 };
    let slot = Slot { current: Some(&mut a) };
    if let Some(r) = slot.current {
        *r = Account { balance: 13 };
    }
    a.balance
}

fn method_option() -> Result<u32, String> {
    let mut ledger = Ledger {
        by_name: HashMap::from([("a".to_string(), Account { balance: 0 })]),
    };
    *ledger.named("a").ok_or("missing")? = Account { balance: 14 };
    Ok(ledger.by_name["a"].balance)
}

fn holder_in_option() -> u32 {
    let mut a = Account { balance: 0 };
    let h = Some(Holder { account: &mut a });
    if let Some(holder) = h {
        *holder.account = Account { balance: 15 };
    }
    a.balance
}

fn set_through(h: &mut Holder) {
    *h.account = Account { balance: 16 };
}

fn through_fn() -> u32 {
    let mut a = Account { balance: 0 };
    let mut h = Holder { account: &mut a };
    set_through(&mut h);
    a.balance
}

fn slot_as_mut() -> u32 {
    let mut a = Account { balance: 0 };
    let mut slot = Slot { current: Some(&mut a) };
    **slot.current.as_mut().unwrap() = Account { balance: 17 };
    a.balance
}

fn tuple_of_field() -> String {
    let mut pair = Pair { a: Point { x: 0 } };
    let q = pair.a;
    let t = (&mut pair.a, 0);
    *t.0 = Point { x: 18 };
    format!("{q:?} {:?}", pair.a)
}

fn option_binding() -> u32 {
    let mut o = Some(Account { balance: 0 });
    if let Some(a) = &mut o {
        *a = Account { balance: 20 };
    }
    o.map_or(0, |a| a.balance)
}

fn block_reborrow() -> u32 {
    let mut a = Account { balance: 0 };
    let h = Holder { account: &mut a };
    *{ &mut *h.account } = Account { balance: 21 };
    a.balance
}

fn holder_of_item() -> String {
    let mut v = vec![Point { x: 0 }];
    let q = v[0];
    let h = PointHolder { point: &mut v[0] };
    *h.point = Point { x: 19 };
    format!("{q:?} {v:?}")
}

fn main() {
    println!("tree {:?}", tree());
    println!("hash {:?}", hash());
    println!("option {:?}", option());
    println!("holder {}", holder());
    println!("vec_item {}", vec_item());
    println!("tuple {}", tuple());
    println!("vec_of_refs {}", vec_of_refs());
    println!("nested {}", nested());
    println!("copy_first {:?}", copy_first());
    println!("copy_in_map {:?}", copy_in_map());
    println!("holder_moved_on {}", holder_moved_on());
    println!("slot_field {}", slot_field());
    println!("method_option {:?}", method_option());
    println!("holder_in_option {}", holder_in_option());
    println!("through_fn {}", through_fn());
    println!("slot_as_mut {}", slot_as_mut());
    println!("tuple_of_field {}", tuple_of_field());
    println!("holder_of_item {}", holder_of_item());
    println!("option_binding {}", option_binding());
    println!("block_reborrow {}", block_reborrow());
}
