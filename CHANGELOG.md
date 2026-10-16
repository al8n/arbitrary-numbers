# UNRELEASED

FEATURES

- `Decimal<Integral, Fraction, Repr>`: exact fixed-point decimal numbers with type-level digit
  counts, stored in a native integer from `i8` to `i128` or from `u8` to `u128` (up to 38 digits).
  - `const fn` constants and methods for each storage: `ZERO`, `ONE`, `MIN`, `MAX`, `parse`,
    `from_repr`, `repr`, `checked_add`, `checked_sub`, `is_zero` and `is_positive`, plus `neg`,
    `abs` and `is_negative` for signed storages.
  - `exact_mul`, `widen` and `try_convert` between decimal types.
  - Exact parsing through `FromStr`, and the shortest exact text through `Display` and `Debug`.
  - Unsigned storages from `u8` to `u128`, named explicitly (`u64` holds 19 digits, one more than
    `i64`): values are never negative, parsing rejects any `-`, and `checked_sub` returns `None`
    below zero. The product of two unsigned storages is unsigned, `widen` never converts a signed
    storage to an unsigned one, and `try_convert` reports a negative value as `OutOfRange`.
- `Storage`, `DigitCount`, `FitsDigits`, `Layout`, `Widen`, `WidenRepr` and `Pick`: the type-level
  capacity and sign checks, with compile-time diagnostics for invalid digit counts and storages.
