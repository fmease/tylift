# TyLift

[![crate](https://img.shields.io/crates/v/tylift.svg)](https://crates.io/crates/tylift)
[![documentation](https://docs.rs/tylift/badge.svg)](https://docs.rs/tylift)
[![rustc](https://img.shields.io/badge/rustc-1.32+-red.svg)](https://blog.rust-lang.org/2019/01/17/Rust-1.32.0.html)
[![license](https://img.shields.io/github/license/fmease/tylift.svg)](https://crates.io/crates/tylift/)

Lift enum variants to the type-level by simply adding the attribute `tylift`.
This comes in handy for type-level programming. The crate you are looking at is
brand new and far from polished!

**Important note**: This library provides mechanisms nearly identical to the experimental
feature [const generics](https://github.com/rust-lang/rfcs/blob/master/text/2000-const-generics.md) which
has not been fully implemented yet. See respective section below on why this crate stays relevant nonetheless.

The attribute promotes variants to their own types which will **not** be namespaced
by current design. The enum type becomes a kind emulated by a trait. In the
process, the original type gets replaced. In Rust, the syntax of trait bounds (`:`) beautifully
mirror the syntax of type annotations. Thus, the snippet `B: Bool` can also be
read as "type parameter `B` of kind `Bool`".

As of right now, there is no automated way to reify the lifted variants. Variants can hold
unnamed fields of types of given kind. Lifted enum types cannot be generic over kinds.
The promoted variants inherit the visibility of the lifted enum. Traits representing kinds
are sealed, which means nobody is able to add new types to the kind.

Attributes (notably documentation comments) applied to the item itself and its variants will be
preserved. On the other hand, attributes placed in front of fields of a variant
(constructor arguments) will not be translated and thus have no effect.
Explicit discriminants are ignored, too.

## First Example

```rust
use tylift::tylift;
use std::marker::PhantomData;

#[tylift]
pub enum Mode {
    Safe,
    Fast,
}

pub struct Text<M: Mode> {
    content: String,
    _marker: PhantomData<M>,
}

impl<M: Mode> Text<M> {
    pub fn into_inner(self) -> String {
        self.content
    }
}

impl Text<Safe> {
    pub fn from(content: Vec<u8>) -> Option<Self> {
        Some(Self {
            content: String::from_utf8(content).ok()?,
            _marker: PhantomData,
        })
    }
}

impl Text<Fast> {
    pub unsafe fn from(content: Vec<u8>) -> Self {
        Self {
            content: String::from_utf8_unchecked(content),
            _marker: PhantomData,
        }
    }
}

fn main() {
    let safe = Text::<Safe>::from(vec![0x73, 0x61, 0x66, 0x65]);
    let fast = unsafe { Text::<Fast>::from(vec![0x66, 0x61, 0x73, 0x74]) };
    assert_eq!(safe.map(Text::into_inner), Some("safe".to_owned()));
    assert_eq!(fast.into_inner(), "fast".to_owned());
}
```

## Installation

Works with `rustc` version 1.32 (stable) or above, Rust 2018 edition. Add these lines to your `Cargo.toml`:

```toml
[dependencies]
tylift = "0.3.1"
```

## More Examples

Code before the macro expansion:

```rust
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
    Branch(BinaryTree, Nat, BinaryTree),
}
```

And after:

```rust
use tylift::tylift;

pub use __tylift_enum_Bool::*;
mod __tylift_enum_Bool {
    use super::*;
    pub trait Bool: __sealed::__Sealed {}
    pub struct False(::core::marker::PhantomData<()>);
    impl Bool for False {}
    pub struct True(::core::marker::PhantomData<()>);
    impl Bool for True {}
    mod __sealed {
        use super::*;
        pub trait __Sealed {}
        impl __Sealed for False {}
        impl __Sealed for True {}
    }
}

pub(crate) use __tylift_enum_Nat::*;
mod __tylift_enum_Nat {
    use super::*;
    pub trait Nat: __sealed::__Sealed {}
    pub struct Zero(::core::marker::PhantomData<()>);
    impl Nat for Zero {}
    pub struct Succ<__T0: Nat>(::core::marker::PhantomData<(__T0)>);
    impl<__T0: Nat> Nat for Succ<__T0> {}
    mod __sealed {
        use super::*;
        pub trait __Sealed {}
        impl __Sealed for Zero {}
        impl<__T0: Nat> __Sealed for Succ<__T0> {}
    }
}

use __tylift_enum_BinaryTree::*;
mod __tylift_enum_BinaryTree {
    use super::*;
    pub trait BinaryTree: __sealed::__Sealed {}
    pub struct Leaf(::core::marker::PhantomData<()>);
    impl BinaryTree for Leaf {}
    pub struct Branch<__T0: BinaryTree, __T1: Nat, __T2: BinaryTree>(
        ::core::marker::PhantomData<(__T0, __T1, __T2)>,
    );
    impl<__T0: BinaryTree, __T1: Nat, __T2: BinaryTree> BinaryTree for Branch<__T0, __T1, __T2> {}
    mod __sealed {
        use super::*;
        pub trait __Sealed {}
        impl __Sealed for Leaf {}
        impl<__T0: BinaryTree, __T1: Nat, __T2: BinaryTree> __Sealed for Branch<__T0, __T1, __T2> {}
    }
}
```

## TyLift Versus Const Generics

Advantages of this crate over const generics:

* recursive kinds which cannot be represented with const generics right now. The latter would also require
  explicit boxing
* once tyfns (type-lifted functions) are implemented, TyLift features pattern matching on types
  \[Todo: Add source on restrictions of const generics\]
* \[Todo\]

## Future Plans

* replacing the introductery example with something more reasonable
* creating tests
* enhancing the generated names for friendlier error messages
* adding additional features like
  * type-level functions
  * generation of a reification function
* removing the feature-gate `span_errors` once `proc_macro_diagnostic` becomes stable

