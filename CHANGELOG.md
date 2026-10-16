# UNRELEASED

FEATURES

- `Decimal<Integral, Fraction, Repr>`: exact fixed-point decimal numbers with type-level digit
  counts, stored in a native signed integer from `i8` to `i128` (up to 38 digits).
  - `const fn` constants and methods for each storage: `ZERO`, `ONE`, `MIN`, `MAX`, `parse`,
    `from_repr`, `repr`, `checked_add`, `checked_sub`, `neg`, `abs`, `is_zero`, `is_negative` and
    `is_positive`.
  - `exact_mul`, `widen` and `try_convert` between decimal types.
  - Exact parsing through `FromStr`, and the shortest exact text through `Display` and `Debug`.
- `Storage`, `DigitCount`, `FitsDigits`, `Layout` and `Widen`: the type-level capacity checks, with
  compile-time diagnostics for invalid digit counts and storages.
