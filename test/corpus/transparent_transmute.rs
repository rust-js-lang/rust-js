// A `#[repr(transparent)]` struct is its one field, the same JS value
// (ADR 0584): a `transmute` between them, by value or through a reference,
// as uuid's `from_bytes_ref` and `as_hyphenated` do (ADR 0448), is the
// value itself, and a `&mut` writes the one value both are.
use std::fmt;
use std::marker::PhantomData;

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

/// A closure's newtype, as React's scheduler's `Callback`, which a type
/// alias can't be, being of itself.
#[repr(transparent)]
struct Callback(Box<dyn Fn(u32) -> Option<Callback>>);

/// A struct of a field and a marker of no size.
#[repr(transparent)]
struct Tagged<T> {
    _kind: PhantomData<T>,
    value: u32,
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
    let mut blank = Uuid([0; 4]);
    let written: &mut [u8; 4] = unsafe { std::mem::transmute::<&mut Uuid, &mut [u8; 4]>(&mut blank) };
    written[0] = 7;
    println!("{blank:?} {}", std::ptr::eq(id.as_bytes(), &bytes));
    let countdown = Callback(Box::new(|n| (n > 0).then(|| Callback(Box::new(|_| None)))));
    let next = (countdown.0)(2).expect("more");
    println!("{} {}", (next.0)(1).is_none(), (countdown.0)(0).is_none());
    let tagged: Tagged<String> = Tagged { _kind: PhantomData, value: 3 };
    println!("{}", tagged.value + 1);
    let mut replaced = String::new();
    let whole = unsafe { std::mem::transmute::<&mut String, &mut Named>(&mut replaced) };
    *whole = Named { name: String::from("x") };
    println!("{replaced}");
    let mut items = vec![1];
    widened(&mut items).push(2);
    println!("{items:?}");
    let mut text = String::from("a");
    longer(&mut text).push('b');
    println!("{text}");
}
