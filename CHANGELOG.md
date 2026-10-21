# UNRELEASED

FEATURES

- `Decimal<Integral, Fraction, Repr>`: exact fixed-point decimal numbers with type-level digit
  counts, stored in a native integer from `i8` to `i128` or from `u8` to `u128` (up to 38 digits),
  or in a wide integer from `i192` to `i320` or from `u192` to `u320` (up to 96 digits).
  - `const fn` constants and methods for each storage: `ZERO`, `ONE`, `MIN`, `MAX`, `parse`,
    `from_repr`, `repr`, `checked_add`, `checked_sub`, `is_zero` and `is_positive`, plus `neg`,
    `abs` and `is_negative` for signed storages.
  - `exact_mul`, `widen` and `try_convert` between decimal types.
  - Exact parsing through `FromStr`, and the shortest exact text through `Display` and `Debug`.
  - Parsing accepts the numeric strings of the General Decimal Arithmetic specification, every
    form that `f64` parses (signs, `.5`, `5.`, exponents) and `_` separators as in Rust literals,
    and reports infinity and NaN as the new `ParseErrorKind::NotFinite`. Exponents of any size are
    applied exactly.
  - Unsigned storages from `u8` to `u128`, named explicitly (`u64` holds 19 digits, one more than
    `i64`): values are never negative, parsing rejects a leading `-`, and `checked_sub` returns
    `None` below zero. The product of two unsigned storages is unsigned, `widen` never converts a
    signed storage to an unsigned one, and `try_convert` reports a negative value as `OutOfRange`.
  - Wide storages: `i192`, `i256` and `i320` hold 57, 76 and 96 digits, and `u192`, `u256` and
    `u320` hold 57, 77 and 96, so any uint256 amount fits `Decimal<U60, U18, u320>`. They have the
    same constants, `const fn` methods, parsing and formatting as the native storages; products
    reach up to 96 digits, and `widen` and `try_convert` convert between native and wide storages.
- `Uint<Bits, Repr>` and `Int<Bits, Repr>`, with the aliases `u192`, `u256`, `u320`, `i192`, `i256`
  and `i320` over 64-bit limbs: `const fn` `from_limbs` and `to_limbs` (least significant limb
  first, two's complement for `Int`), `ZERO`, numeric `Eq`, `Ord` and `Hash`, and decimal
  `Display` and `Debug`.
- `Storage`, `DigitCount`, `FitsDigits`, `Layout`, `Widen`, `WidenRepr` and `Pick`: the type-level
  capacity and sign checks, with compile-time diagnostics for invalid digit counts and storages.
