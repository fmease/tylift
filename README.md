# TyLift

[![crate](https://img.shields.io/crates/v/tylift.svg)](https://crates.io/crates/tylift)
[![documentation](https://docs.rs/tylift/badge.svg)](https://docs.rs/tylift)
[![license](https://img.shields.io/github/license/fmease/tylift.svg)](https://crates.io/crates/tylift/)

Lift enum variants to the type-level by simply adding the attribute `tylift`.
This comes in handy for type-level programming.

**Important note**: This library provides mechanisms nearly identical to the experimental
feature [const generics](https://github.com/rust-lang/rfcs/blob/master/text/2000-const-generics.md) which
has not been fully implemented yet. See respective section below on why this crate stays relevant nonetheless.

The attribute promotes variants to their own types which will are by default not namespaced.
The enum type becomes a _kind_ emulated by a trait. In the process, the original type gets replaced.
In Rust, the syntax of trait bounds (`:`) beautifully mirror the syntax of type annotations.
Thus, the snippet `B: Bool` can also be read as "type parameter `B` of kind `Bool`".

As of right now, there is no automated way to reify the lifted variants. Variants can hold
unnamed fields of types of given kind. Lifted enum types cannot be generic over kinds.
The promoted variants inherit the visibility of the lifted enum. Traits representing kinds
are sealed, which means nobody is able to add new types to the kind.

Attributes (notably documentation comments) applied to the item itself and its variants will be
preserved. On the other hand, attributes placed in front of fields of a variant
(constructor arguments) will not be translated and thus have no effect.
Explicit discriminants are ignored, too.

Expanded code works in `#![no_std]`-environments.

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
            content: unsafe { String::from_utf8_unchecked(content) },
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

Add these lines to your `Cargo.toml`:

```toml
[dependencies]
tylift = "0.3.4"
```

Compability with older `rustc` versions is currently not verified. Older versions of this crate (≤ 0.3.2) only
relied on features of `rustc` 1.32. So you might want to check them out.

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

#[tylift(mod)] // put all 3 items into the module `Power`
pub enum Power {
    On,
    Off,
}

#[tylift(mod direction)] // put all 3 items into the module `direction`
pub(crate) enum Direction {
    Up,
    Down,
}
```

And after expansion below. It's partially hygienic, identifiers prefixed with double underscores are unhygienic.
The reason is current API limitations of `proc_macro`.

```rust
use tylift::tylift;

pub use __kind_Bool::*;
mod __kind_Bool {
    use super::*;
    pub trait Bool: sealed::Sealed {}
    pub struct False(::core::marker::PhantomData<()>);
    impl Bool for False {}
    pub struct True(::core::marker::PhantomData<()>);
    impl Bool for True {}
    mod sealed {
        use super::*;
        pub trait Sealed {}
        impl Sealed for False {}
        impl Sealed for True {}
    }
}

pub(crate) use __kind_Nat::*;
mod __kind_Nat {
    use super::*;
    pub trait Nat: sealed::Sealed {}
    pub struct Zero(::core::marker::PhantomData<()>);
    impl Nat for Zero {}
    pub struct Succ<T0: Nat>(::core::marker::PhantomData<(T0)>);
    impl<T0: Nat> Nat for Succ<T0> {}
    mod sealed {
        use super::*;
        pub trait Sealed {}
        impl Sealed for Zero {}
        impl<T0: Nat> Sealed for Succ<T0> {}
    }
}

use __kind_BinaryTree::*;
mod __kind_BinaryTree {
    use super::*;
    pub trait BinaryTree: sealed::Sealed {}
    pub struct Leaf(::core::marker::PhantomData<()>);
    impl BinaryTree for Leaf {}
    pub struct Branch<T0: BinaryTree, T1: Nat, T2: BinaryTree>(
        ::core::marker::PhantomData<(T0, T1, T2)>,
    );
    impl<T0: BinaryTree, T1: Nat, T2: BinaryTree> BinaryTree for Branch<T0, T1, T2> {}
    mod sealed {
        use super::*;
        pub trait Sealed {}
        impl Sealed for Leaf {}
        impl<T0: BinaryTree, T1: Nat, T2: BinaryTree> Sealed for Branch<T0, T1, T2> {}
    }
}

pub mod Power {
    use super::*;
    pub trait Power: sealed::Sealed {}
    pub struct On(::core::marker::PhantomData<()>);
    impl Power for On {}
    pub struct Off(::core::marker::PhantomData<()>);
    impl Power for Off {}
    mod sealed {
        use super::*;
        pub trait Sealed {}
        impl Sealed for On {}
        impl Sealed for Off {}
    }
}

pub(crate) mod direction {
    use super::*;
    pub trait Direction: sealed::Sealed {}
    pub struct Up(::core::marker::PhantomData<()>);
    impl Direction for Up {}
    pub struct Down(::core::marker::PhantomData<()>);
    impl Direction for Down {}
    mod sealed {
        use super::*;
        pub trait Sealed {}
        impl Sealed for Up {}
        impl Sealed for Down {}
    }
}
```

### Manually Writing a Function

```rust
// for kind `Bool` from above
type Not<B> = <B as NotImpl>::Result;
trait NotImpl: Bool { type Result: Bool; }
impl NotImpl for False { type Result = True; }
impl NotImpl for True { type Result = False; }

// for kind `Nat` from above
type Add<N, M> = <N as AddImpl<M>>::Result;
trait AddImpl<M: Nat>: Nat { type Result: Nat }
impl<M: Nat> AddImpl<M> for Zero { type Result = M; }
impl<N: Nat, M: Nat> AddImpl<M> for Succ<N>
// where clause necessary because the type system does not know that
// the trait is sealed (only the module system knows)
where N: AddImpl<Succ<M>>
{
    type Result = Add<N, Succ<M>>;
}
```

## TyLift Versus Const Generics

Advantages of this crate over const generics:

* recursive kinds which cannot be represented with const generics right now.
  The latter would also require _explicit boxing_
* \[not sure what else\]

## Future Plans

* replacing the introductery example with something more reasonable
* creating tests
* adding additional features like
  * type-level functions
  * generation of a reification function
* removing the feature-gate `span_errors` once `proc_macro_diagnostic` becomes stable

