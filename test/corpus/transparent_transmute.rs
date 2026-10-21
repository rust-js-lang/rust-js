// A `#[repr(transparent)]` struct is its one field: a `transmute` between
// them, by value or through a `&`, makes one of the other or reads its
// field, as uuid's `from_bytes_ref` and `as_hyphenated` do (ADR 0448).
use std::fmt;

#[repr(transparent)]
#[derive(Debug, PartialEq)]
struct Uuid([u8; 4]);

#[repr(transparent)]
struct Hyphenated(Uuid);

#[repr(transparent)]
struct Named {
    name: String,
}

impl Uuid {
    fn from_bytes_ref(bytes: &[u8; 4]) -> &Uuid {
        unsafe { std::mem::transmute::<&[u8; 4], &Uuid>(bytes) }
    }

    fn as_bytes(&self) -> &[u8; 4] {
        unsafe { std::mem::transmute::<&Uuid, &[u8; 4]>(self) }
    }

    fn as_hyphenated(&self) -> &Hyphenated {
        unsafe { std::mem::transmute::<&Uuid, &Hyphenated>(self) }
    }
}

impl fmt::Display for Hyphenated {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let [a, b, c, d] = (self.0).0;
        write!(f, "{a:02x}{b:02x}-{c:02x}{d:02x}")
    }
}

/// The same type, but for its lifetime: itself, a `&mut` too.
fn widened(items: &mut Vec<u32>) -> &mut Vec<u32> {
    unsafe { std::mem::transmute::<&mut Vec<u32>, &mut Vec<u32>>(items) }
}

/// A `&mut` given a longer lifetime, as yoke erases one: itself.
fn longer<'a>(text: &'a mut String) -> &'static mut String {
    unsafe { std::mem::transmute::<&'a mut String, &'static mut String>(text) }
}

fn main() {
    let bytes = [1, 2, 0xab, 0xcd];
    let id = Uuid::from_bytes_ref(&bytes);
    println!("{:?} {}", id, id.as_hyphenated());
    println!("{:?}", id.as_bytes());
    let inner: &[u8; 4] = unsafe { std::mem::transmute::<&Hyphenated, &[u8; 4]>(id.as_hyphenated()) };
    println!("{}", inner[3]);
    let owned: Uuid = unsafe { std::mem::transmute::<[u8; 4], Uuid>([9, 8, 7, 6]) };
    println!("{:?} {}", owned, owned == Uuid([9, 8, 7, 6]));
    let named: Named = unsafe { std::mem::transmute::<String, Named>(String::from("ada")) };
    let text: &String = unsafe { std::mem::transmute::<&Named, &String>(&named) };
    println!("{} {}", named.name, text.len());
    let mut items = vec![1];
    widened(&mut items).push(2);
    println!("{items:?}");
    let mut text = String::from("a");
    longer(&mut text).push('b');
    println!("{text}");
}
