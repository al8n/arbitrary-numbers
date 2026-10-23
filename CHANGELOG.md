# UNRELEASED

FEATURES

- `Decimal<Integral, Fraction, Repr>`: exact fixed-point decimal numbers with type-level digit
  counts, stored in a native integer from `i8` to `i128` or from `u8` to `u128` (up to 38 digits),
  or in a wide integer from `i192` to `i512` or from `u192` to `u512` (up to 153 digits, and 154
  with an explicit `u512`).
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
  - Wide storages: `i192`, `i256`, `i320`, `i384`, `i448` and `i512` hold 57, 76, 96, 115, 134 and
    153 digits, and `u192`, `u256`, `u320`, `u384`, `u448` and `u512` hold 57, 77, 96, 115, 134 and
    154, so any uint256 amount fits `Decimal<U60, U18, u320>`. They have the same constants,
    `const fn` methods, parsing and formatting as the native storages; products reach up to 153
    digits, and `widen` and `try_convert` convert between native and wide storages. The 154th digit
    is available only with an explicit `u512`, as in `Decimal<U154, U0, u512>`, because no signed
    storage holds it.
- `RoundingMode`, with the modes `Up`, `Down`, `Ceiling`, `Floor`, `HalfUp`, `HalfDown` and
  `HalfEven`. Rounding happens only where a mode is given, for native and wide storages alike:
  - `round_to` converts to any decimal type as `try_convert` does, but rounds dropped fraction
    digits with the mode instead of failing. It reports `OutOfRange` if the rounded value does not
    fit, so a negative value converts to an unsigned storage only when it rounds to zero.
  - `const fn` `quantize` rounds to a multiple of a step of the same type with the mode, and reports
    the new `QuantizeError`: `InvalidStep` for a step that is zero or negative, and `OutOfRange` for
    a multiple outside the range.
  - `div_round` divides by a decimal of any type into any decimal type, rounding the quotient once
    with the mode, and reports the new `DivideError`: `DivisionByZero` for a zero divisor, which
    never gives zero, and `OutOfRange` for a quotient outside the target's range. The intermediate
    is sized at the type level; one of more than 153 digits fails to compile, and 154 is allowed
    only when both storages are unsigned.
  - `const fn` `is_multiple_of` checks a value against a step without rounding it.
- The `serde` feature: `Decimal` is written as its text and read from a string, never from a number,
  in every format, and `RoundingMode` as the name of its variant, without allocation. The impls use
  `serde_core`, so they need `serde` 1.0.220 or later in the build; the feature enables no feature
  of any other crate.
- The `arbitrary`, `proptest` and `quickcheck` features: the `Arbitrary` trait of each crate for
  `Decimal` and `RoundingMode`, for fuzzing and property tests on a target with `std`. One generator
  serves all three: an edge value one time in eight (zero, one unit in the last place, one, `MAX`,
  powers of ten, runs of nines and the rounding ties `5 * 10^k` and `15 * 10^k`), and otherwise a
  number of digits chosen uniformly and then uniform digits, with trailing zeros one time in four,
  so that every magnitude is common. `proptest` and `quickcheck` shrink a decimal by truncating its
  digits and a rounding mode toward `Down`. The `proptest` feature adds `numo::proptest`, with
  `DecimalStrategy` and `DecimalValueTree`, and the `quickcheck` feature allocates only the boxed
  iterator that its trait requires shrinking to return.
- `Uint<Bits, Repr>` and `Int<Bits, Repr>`, with the aliases `u192`, `u256`, `u320`, `u384`, `u448`,
  `u512`, `i192`, `i256`, `i320`, `i384`, `i448` and `i512` over 64-bit limbs: `const fn`
  `from_limbs` and `to_limbs` (least significant limb first, two's complement for `Int`), `ZERO`,
  numeric `Eq`, `Ord` and `Hash`, and decimal `Display` and `Debug`.
- `Storage`, `DigitCount`, `FitsDigits`, `Layout`, `Widen`, `WidenRepr` and `Pick`: the type-level
  capacity and sign checks, with compile-time diagnostics for invalid digit counts and storages.
