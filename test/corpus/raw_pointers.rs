// A raw pointer is a handle on its place, one for each place, or `null`:
// `*p` reads and writes the place, `p == q` is whether it's the same one,
// and a local a pointer is taken to is a cell (ADR 0407).
use std::ops::{Deref, DerefMut};
use std::ptr;

fn bump(p: *mut i32) {
    unsafe {
        *p += 1;
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Pt {
    x: i32,
    y: i32,
}

// A list linked by raw pointers.
struct Node {
    value: u32,
    next: *mut Node,
}

fn sum(mut at: *const Node) -> u32 {
    let mut total = 0;
    while !at.is_null() {
        unsafe {
            total += (*at).value;
            at = (*at).next;
        }
    }
    total
}

// A pointer a generic type holds, read through `Deref`.
struct Own<T> {
    value: *mut T,
}

impl<T> Deref for Own<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.value }
    }
}

impl<T> DerefMut for Own<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value }
    }
}

// Only pointed to, never changed: its copy is a copy still.
#[derive(Clone, Copy)]
struct Tag {
    id: u32,
}

fn same(a: &Pt, b: &Pt) -> bool {
    ptr::eq(a, b)
}

// A pointer nothing reads, of what a shared reference holds, which has no
// place: made of nothing.
fn unread(n: &i32) -> i32 {
    let _p: *const i32 = n;
    *n
}

// A `ref mut` binding read through a match guard, the `&mut` a value of its
// own there.
fn guarded(mut n: i32) -> i32 {
    let r;
    match n {
        ref mut z if {
            r = z as &_;
            true
        } =>
        {
            *r
        }
        _ => 1,
    }
}

fn main() {
    let mut x = 123;
    let c = &raw const x;
    let m = &raw mut x;
    let r = &x;
    let copied = r;
    println!("{} {} {}", c == r as *const i32, c == m as *const i32, copied as *const i32 == c);
    unsafe {
        *m = 5;
    }
    println!("{} {}", x, unsafe { *c });
    bump(&mut x);
    println!("{}", x);

    let mut pt = Pt { x: 1, y: 2 };
    let px = &mut pt.x as *mut i32;
    unsafe {
        *px = 10;
    }
    let pp = &mut pt as *mut Pt;
    unsafe {
        (*pp).y = 20;
        *pp = Pt { x: (*pp).x + 1, y: (*pp).y };
    }
    println!("{:?}", pt);

    let n: *const i32 = ptr::null();
    println!("{} {}", n.is_null(), c.is_null());
    unsafe {
        println!("{:?} {:?}", n.as_ref(), c.as_ref());
        let old = ptr::replace(m, 40);
        println!("{} {}", old, ptr::read(c));
        ptr::write(m, 41);
        let mut y = 7;
        ptr::swap(m, &mut y);
        println!("{} {}", x, y);
        if let Some(z) = m.as_mut() {
            *z += 1;
        }
        println!("{}", x);
    }

    let mut third = Node { value: 3, next: ptr::null_mut() };
    let mut second = Node { value: 2, next: &mut third };
    let first = Node { value: 1, next: &mut second };
    println!("{}", sum(&first));
    third.value = 30;
    println!("{}", sum(&first));

    let mut point = Pt { x: 0, y: 0 };
    let mut own = Own { value: &mut point };
    own.x = 5;
    own.y += 2;
    println!("{:?} {}", point, (*own).x);

    let a = Pt { x: 1, y: 1 };
    let b = a;
    println!("{} {} {}", same(&a, &b), same(&a, &a), a == b);

    println!("{} {}", unread(&4), guarded(6));
    let literal: *const i32 = &3;
    println!("{}", unsafe { *literal });
    let once = 5;
    let at_once = &raw const once;
    println!("{}", unsafe { *at_once });
    let again = unsafe { &raw const *c };
    println!("{}", again == c);

    let tag = Tag { id: 1 };
    let other = tag;
    println!("{} {}", ptr::eq(&tag, &other), tag.id == other.id);
    // What holds a pointer compares it by its place, not what it points to.
    let mut twin = 5;
    let elsewhere = &raw mut twin;
    let (seven, also_seven) = (7, 7);
    println!(
        "{} {} {}",
        Some(c) == Some(m),
        Some(c) == Some(elsewhere as *const i32),
        Some(&raw const seven) == Some(&raw const also_seven)
    );
    let nothing: Option<i32> = None;
    let at_nothing = &raw const nothing;
    println!("{:?}", unsafe { at_nothing.as_ref() });

    let items = [10, 20, 30];
    let start = items.as_ptr();
    let first_item = &items[0] as *const i32;
    unsafe {
        println!("{} {} {}", *start.add(2), *first_item.add(1), start == first_item);
        let last = start.add(2);
        let via = &items as *const i32;
        println!("{} {} {}", *last.sub(1), via == start, *via.cast::<i32>());
    }
    let boxed = Box::into_raw(Box::new(Pt { x: 7, y: 8 }));
    let number = Box::into_raw(Box::new(9));
    unsafe {
        (*boxed).x += 1;
        *number += 1;
        println!("{:?} {}", *Box::from_raw(boxed), *Box::from_raw(number));
    }
    println!("{:?}", ptr::null::<u8>());
}
