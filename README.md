# TyLift

Lift enum variants to the type-level by simply adding the attribute `tylift`.
This comes in handy for type-level programming. The crate you are looking at is
brand new and far from polished!

The attribute promotes variants to their own types which will not be namespaced
by current design. The enum type becomes a kind emulated by a trait. In the
process, the original type gets replaced. The syntax of trait bounds (`:`) beautifully
mirror the syntax of type annotations in Rust. Thus, the snippet `B: Bool` can also be
read as "type parameter `B` of kind `Bool`".

As of right now, there is no automated way to reify the lifted variants. Variants can hold
unnamed fields of types of given kind. Lifted enum types (kinds) cannot be generic over
kinds (yet). The promoted variants inherit the visibility of the lifted enum. Another issue
concerns the implementation of kinds: They are not protected from being extended by parties who
imported them. At least, they require an `unsafe impl` to be enlarged.

## First Example

```rust
#![feature(never_type)]
use tylift::tylift;
use std::marker::PhantomData;

#[tylift]
pub enum Mode {
    Slow,
    Normal,
    Fast
}

pub struct Machine<M: Mode>(PhantomData<M>);

impl<M: Mode> Machine<M> {
    fn new() -> Self { Machine(PhantomData) }
}

impl Machine<Slow> {
    fn motivate(&self) -> Result<(), ()> {
        unimplemented!()
    }
}

impl Machine<Fast> {
    fn keep_at_bay(&self) -> Result<(), ()> {
        unimplemented!()
    }
}
```

## Installation

Currently only tested with `rustc` 1.33 (nightly), 2018 edition. Requires the
experimental feature `never_type`. Add these lines to your `Cargo.toml`:

```toml
[dependencies]
tylift = "0.1.0"
```

## More Examples

Code before the macro expansion:

```rust
#![feature(never_type)]
use tylift::tylift;

#[tylift]
pub enum Bool {
    False,
    True,
}

#[tylift]
enum Nat {
    Zero,
    Succ(Nat),
}

#[tylift]
enum NatBTree {
    Leaf,
    Branch(NatBTree, Nat, NatBTree),
}
```

And after:

```rust
#![feature(never_type)]
use tylift::tylift;

pub unsafe trait Bool {}
pub struct False(!, ::std::marker::PhantomData<()>);
unsafe impl Bool for False {}
pub struct True(!, ::std::marker::PhantomData<()>);
unsafe impl Bool for True {}

unsafe trait Nat {}
struct Zero(!, ::std::marker::PhantomData<()>);
unsafe impl Nat for Zero {}
struct Succ<__T0: Nat>(!, ::std::marker::PhantomData<(__T0)>);
unsafe impl<__T0: Nat> Nat for Succ<__T0> {}

unsafe trait NatBTree {}
struct Leaf(!, ::std::marker::PhantomData<()>);
unsafe impl NatBTree for Leaf {}
struct Branch<__T0: NatBTree, __T1: Nat, __T2: NatBTree>(
    !,
    ::std::marker::PhantomData<(__T0, __T1, __T2)>,
);
unsafe impl<__T0: NatBTree, __T1: Nat, __T2: NatBTree> NatBTree for Branch<__T0, __T1, __T2> {}
```

## Tasks

* fix the issues mentioned above
* better error messages
* documentation
* tests
* additional features like type-level functions
