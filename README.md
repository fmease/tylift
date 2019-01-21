# TyLift

[![crate](https://img.shields.io/crates/v/tylift.svg)](https://crates.io/crates/tylift)
[![documentation](https://docs.rs/tylift/badge.svg)](https://docs.rs/tylift)
[![license](https://img.shields.io/github/license/fmease/tylift.svg)](https://crates.io/crates/tylift/)

Lift enum variants to the type-level by simply adding the attribute `tylift`.
This comes in handy for type-level programming. The crate you are looking at is
brand new and far from polished!

The attribute promotes variants to their own types which will not be namespaced
by current design. The enum type becomes a kind emulated by a trait. In the
process, the original type gets replaced. The syntax of trait bounds (`:`) beautifully
mirror the syntax of type annotations in Rust. Thus, the snippet `B: Bool` can also be
read as "type parameter `B` of kind `Bool`".

As of right now, there is no automated way to reify the lifted variants. Variants can hold
unnamed fields of types of given kind. Lifted enum types cannot be generic over kinds.
The promoted variants inherit the visibility of the lifted enum. Traits representing kinds
are sealed, which means nobody is able to add new types to the kind.

Attributes applied to the item itself (placed below `tylift`), its variants or fields of its
variants will not be translated and have no effect.

## First Example

```rust
#![feature(never_type)]
use tylift::tylift;
use std::marker::PhantomData;

#[tylift]
pub enum Mode {
    Slow,
    Normal,
    Fast,
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
tylift = "0.2.0"
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
pub(crate) enum Nat {
    Zero,
    Succ(Nat),
}

#[tylift]
enum BinaryTree {
    Leaf,
    Branch(NatBTree, Nat, BinaryTree),
}
```

And after:

```rust
#![feature(never_type)]
use tylift::tylift;

pub use self::__tylift_enum_Bool::*;
mod __tylift_enum_Bool {
    use super::*;
    pub trait Bool: __sealed::__Sealed {}
    pub struct False(!, ::std::marker::PhantomData<()>);
    impl Bool for False {}
    pub struct True(!, ::std::marker::PhantomData<()>);
    impl Bool for True {}
    mod __sealed {
        use super::*;
        pub trait __Sealed {}
        impl __Sealed for False {}
        impl __Sealed for True {}
    }
}

pub(crate) use self::__tylift_enum_Nat::*;
mod __tylift_enum_Nat {
    use super::*;
    pub trait Nat: __sealed::__Sealed {}
    pub struct Zero(!, ::std::marker::PhantomData<()>);
    impl Nat for Zero {}
    pub struct Succ<__T0: Nat>(!, ::std::marker::PhantomData<(__T0)>);
    impl<__T0: Nat> Nat for Succ<__T0> {}
    mod __sealed {
        use super::*;
        pub trait __Sealed {}
        impl __Sealed for Zero {}
        impl<__T0: Nat> __Sealed for Succ<__T0> {}
    }
}

use self::__tylift_enum_NatBTree::*;
mod __tylift_enum_NatBTree {
    use super::*;
    pub trait NatBTree: __sealed::__Sealed {}
    pub struct Leaf(!, ::std::marker::PhantomData<()>);
    impl NatBTree for Leaf {}
    pub struct Branch<__T0: NatBTree, __T1: Nat, __T2: NatBTree>(
        !,
        ::std::marker::PhantomData<(__T0, __T1, __T2)>,
    );
    impl<__T0: NatBTree, __T1: Nat, __T2: NatBTree> NatBTree for Branch<__T0, __T1, __T2> {}
    mod __sealed {
        use super::*;
        pub trait __Sealed {}
        impl __Sealed for Leaf {}
        impl<__T0: NatBTree, __T1: Nat, __T2: NatBTree> __Sealed for Branch<__T0, __T1, __T2> {}
    }
}
```

## Tasks and Plans

* improve the error messages
* write documentation
* create tests
* add additional features like
  * type-level functions
  * reify-function generation
