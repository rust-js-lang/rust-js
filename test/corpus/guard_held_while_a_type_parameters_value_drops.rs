// A guard that's a call's receiver, held while its argument drops a value
// of a type parameter, or of a trait's `Self` in its default, whose
// destructor asks of the guard's cell: each cell was taken for one no borrow
// of which meets another, as what a type parameter's value drops was taken
// for nothing where its body is walked with no drops given, so its borrow
// wasn't counted and the destructor's `try_borrow()` was `Ok` (#1321). Each
// shape has a cell of a type of its own, and a trait of its own, as a
// borrow counted of one type counts every cell of it, a generic one's too.
use std::cell::RefCell;
use std::rc::Rc;

struct Plain(Rc<RefCell<Vec<u8>>>);

impl Drop for Plain {
    fn drop(&mut self) {
        println!("plain: free {}", self.0.try_borrow().is_ok());
    }
}

fn push_after<T>(it: T, counts: &RefCell<Vec<u8>>) {
    counts.borrow_mut().push({
        let _it = it;
        1
    });
}

trait OfConcrete {
    fn push_concrete(self, counts: &RefCell<Vec<u16>>)
    where
        Self: Sized,
    {
        counts.borrow_mut().push({
            let _it = self;
            2
        });
    }
}

struct Concrete(Rc<RefCell<Vec<u16>>>);

impl Drop for Concrete {
    fn drop(&mut self) {
        println!("concrete: free {}", self.0.try_borrow().is_ok());
    }
}

impl OfConcrete for Concrete {}

trait OfGeneric {
    fn push_generic(self, counts: &RefCell<Vec<u64>>)
    where
        Self: Sized,
    {
        counts.borrow_mut().push({
            let _it = self;
            3
        });
    }
}

struct Generic<I>(I, Rc<RefCell<Vec<u64>>>);

impl<I> Drop for Generic<I> {
    fn drop(&mut self) {
        println!("generic: free {}", self.1.try_borrow().is_ok());
    }
}

impl<I> OfGeneric for Generic<I> {}

fn through_generic_code<E: OfGeneric>(it: E, counts: &RefCell<Vec<u64>>) {
    it.push_generic(counts);
}

trait OfConstGeneric {
    fn push_sized(self, counts: &RefCell<Vec<i8>>)
    where
        Self: Sized,
    {
        counts.borrow_mut().push({
            let _it = self;
            4
        });
    }
}

struct Sized_<const N: usize>([u8; N], Rc<RefCell<Vec<i8>>>);

impl<const N: usize> Drop for Sized_<N> {
    fn drop(&mut self) {
        println!("const generic: free {}", self.1.try_borrow().is_ok());
    }
}

impl<const N: usize> OfConstGeneric for Sized_<N> {}

trait OfAny {
    fn push_any(self, counts: &RefCell<Vec<u32>>)
    where
        Self: Sized,
    {
        counts.borrow_mut().push({
            let _it = self;
            5
        });
    }
}

impl<T> OfAny for T {}

struct ByBlanket(Rc<RefCell<Vec<u32>>>);

impl Drop for ByBlanket {
    fn drop(&mut self) {
        println!("blanket: free {}", self.0.try_borrow().is_ok());
    }
}

struct Item(Rc<RefCell<Vec<i16>>>);

impl Drop for Item {
    fn drop(&mut self) {
        println!("item: free {}", self.0.try_borrow().is_ok());
    }
}

fn push_item<I: Iterator>(mut it: I, counts: &RefCell<Vec<i16>>) {
    counts.borrow_mut().push({
        let _first = it.next();
        6
    });
}

struct Captured(Rc<RefCell<Vec<i32>>>);

impl Drop for Captured {
    fn drop(&mut self) {
        println!("captured: free {}", self.0.try_borrow().is_ok());
    }
}

fn push_in_closure<T>(it: T, counts: &RefCell<Vec<i32>>) {
    let run = move || {
        counts.borrow_mut().push({
            let _it = it;
            7
        });
    };
    run();
}

fn main() {
    let plain = Rc::new(RefCell::new(Vec::new()));
    push_after(Plain(plain.clone()), &plain);
    let concrete = Rc::new(RefCell::new(Vec::new()));
    Concrete(concrete.clone()).push_concrete(&concrete);
    let generic = Rc::new(RefCell::new(Vec::new()));
    Generic(1u8, generic.clone()).push_generic(&generic);
    through_generic_code(Generic('x', generic.clone()), &generic);
    let sized = Rc::new(RefCell::new(Vec::new()));
    Sized_([1, 2], sized.clone()).push_sized(&sized);
    let blanket = Rc::new(RefCell::new(Vec::new()));
    ByBlanket(blanket.clone()).push_any(&blanket);
    let item = Rc::new(RefCell::new(Vec::new()));
    push_item(vec![Item(item.clone())].into_iter(), &item);
    let captured = Rc::new(RefCell::new(Vec::new()));
    push_in_closure(Captured(captured.clone()), &captured);
    println!("{:?}", plain.borrow());
    println!("{:?}", concrete.borrow());
    println!("{:?}", generic.borrow());
    println!("{:?}", sized.borrow());
    println!("{:?}", blanket.borrow());
    println!("{:?}", item.borrow());
    println!("{:?}", captured.borrow());
}
