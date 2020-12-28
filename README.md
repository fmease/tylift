# tylift

[![crate](https://img.shields.io/crates/v/tylift.svg)](https://crates.io/crates/tylift)
[![documentation](https://docs.rs/tylift/badge.svg)](https://docs.rs/tylift)
[![license](https://img.shields.io/github/license/fmease/tylift.svg)](https://crates.io/crates/tylift/)

Lift enum variants to the type-level simply by adding the attribute `tylift`. This comes in handy for [type-level programming](https://willcrichton.net/notes/type-level-programming/).

**Important note**: This library provides mechanisms nearly identical to the experimental
feature [const generics](https://github.com/rust-lang/rfcs/blob/master/text/2000-const-generics.md)/[min const genercis](https://github.com/rust-lang/rust/issues/74878) which has not been fully implemented yet. See the respective section below for more information.

The attribute promotes enum variants to their own types. The enum type becomes a [_kind_](https://en.wikipedia.org/wiki/Kind_(type_theory)) – the type of a type – emulated by a trait, replacing the original type declaration. In Rust, the syntax of trait bounds (`:`) beautifully mirror the syntax of type annotations. Thus, the snippet `B: Bool` can also be read as "type parameter `B` of kind `Bool`".

Traits representing kinds are _sealed_, which means nobody is able to add new types to the kind. Variants can hold (unnamed) fields of types of a given kind. Attributes (notably documentation comments) applied to the item itself and its variants will be preserved. Expanded code works in `#![no_std]`-environments.

As of right now, there is no automated way to _reify_ the lifted variants (i.e. map them to their term-level counterpart). Lifted enum types can _not_ be generic over kinds.

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

### Cargo Features

The feature-flag `span_errors` drastically improves error messages by taking advantage of the span information of a token. It uses the experimental feature `proc_macro_diagnostic` and thus requires a nightly `rustc`.

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
    /// Higher and higher!
    Up,
    /// Lower and lower...
    Down,
}
```

And after expansion below. It's partially [hygienic](https://en.wikipedia.org/wiki/Hygienic_macro); generated identifiers which are unhygienic because of current limitations of the `proc_macro` API are prefixed with double underscores (`__`) to lower the change of name collisions.

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
    /// Higher and higher!
    pub struct Up(::core::marker::PhantomData<()>);
    impl Direction for Up {}
    /// Lower and lower...
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

### Manually Writing a Type-Level Function

Type-level function `Not` from kind `Bool` to `Bool` (kind defined in previous section):

```rust
type Not<B> = <B as NotImpl>::Result;

trait NotImpl: Bool { type Result: Bool; }
impl NotImpl for False { type Result = True; }
impl NotImpl for True { type Result = False; }
```

Type-level function `Add` from two `Nat`s to `Nat` (kind defined in previous section):

```rust
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

## tylift Versus Const Generics

**Advantages** of this crate over const generics:

* recursive kinds which cannot be represented with const generics right now.
  The latter would also require _explicit boxing_
* compatibility with older rust versions

Obviously, these are not _that_ convincing arguments. Consider this crate as **a study** rather than something of value. Maybe you can learn from its code.

**Disadvantages**:

* requires an additional dependency (`tylift`) with a heavy transitive dependency on `syn`
* worse tooling
* atrociously hairy type-level functions compared to `const fn`s which are compatible with const generics, see [const evaluatable checked](https://github.com/rust-lang/rust/issues/76560)

## Future Plans

* replacing the introductery example with something more reasonable
* creating tests
* adding additional features like
  * an attribute to lift functions to type-level ones
  * generating reification functions
* removing the feature-gate `span_errors` once `proc_macro_diagnostic` becomes stable
