# bit-struct

[![crates.io](https://img.shields.io/crates/v/bit-struct.svg)](https://crates.io/crates/bit-struct)
[![codecov](https://codecov.io/gh/andrewgazelka/bit-struct/branch/main/graph/badge.svg?token=60R82VBBVF)](https://codecov.io/gh/andrewgazelka/bit-struct)

Bit struct is a crate which allows creating named C-like bit fields without mediocre IDE support resulting from proc macros.

## Benefits
- No proc macros
- Autocompletion fully works (tested in IntelliJ Rust)
- Fast compile times
- Statically checked bounds (structs cannot be over-filled)
- Statically checked types

## Examples

```rust

bit_struct::enums! {
    /// The default value for each enum is always the first
    pub ThreeVariants { Zero, One, Two }

    /// This is syntax to set the default value to Cat
    pub Animal(Cat) { Cow, Bird, Cat, Dog }

    pub Color { Orange, Red, Blue, Yellow, Green }
}

bit_struct::bit_struct! {
    /// We can write documentation for the struct here. Here BitStruct1
    /// derives default values from the above enums macro
    #[derive(Default)]
    struct BitStruct1 (u16){
        /// a 1 bit element. This is stored in u16[15]
        a: bit_struct::u1,

        /// This is calculated to take up 2 bits. This is stored in u16[13..=14]
        variant: ThreeVariants,

        /// This also takes 2 bits. This is stored in u16[11..=12]
        animal: Animal,

        /// This takes up 3 bits. This is stored u16[8..=10]
        color: Color,
    }

    struct BitStruct2(u32) {
        /// We could implement for this too. Note, this does not have a default
        a_color: Color,
        b: bit_struct::u3,
    }
}

fn main() {
    let mut bit_struct: BitStruct1 = BitStruct1::default();

    assert_eq!(bit_struct.a().start(), 15);
    assert_eq!(bit_struct.a().stop(), 15);

    assert_eq!(bit_struct.color().start(), 8);
    assert_eq!(bit_struct.color().stop(), 10);

    assert_eq!(format!("{:?}", bit_struct), "BitStruct1 { a: 0, variant: Zero, animal: Cat, color: Orange }");
    assert_eq!(bit_struct.raw(), 4096);

    let reverse_bit_struct = BitStruct1::try_from(4096);
    assert_eq!(format!("{:?}", reverse_bit_struct), "Ok(BitStruct1 { a: 0, variant: Zero, animal: Cat, color: Orange })");

    // u3! macro provides a static assert that the number is not too large
    let mut other_struct = BitStruct2::new(Color::Green, bit_struct::u3!(0b101));
    assert_eq!(format!("{:?}", other_struct), "BitStruct2 { a_color: Green, b: 5 }");

    assert_eq!(other_struct.a_color().get(), Color::Green);

    other_struct.a_color().set(Color::Red);

    assert_eq!(other_struct.a_color().get(), Color::Red);
}
```
