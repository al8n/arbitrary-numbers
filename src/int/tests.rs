// The modules generated here shadow the primitive type names in this module, so primitive types
// are named only inside them.

macro_rules! core_tests {
  ($($sign:ident $int:ident),+ $(,)?) => {$(
    mod $int {
      use std::{string::ToString, vec::Vec};

      use crate::int::{$int as native, DIGIT_BUF_LEN};

      /// Whether `10^exp` is representable.
      fn representable(exp: u32) -> bool {
        10u128.checked_pow(exp).is_some_and(|power| power <= <$int>::MAX as u128)
      }

      /// The largest `d` such that `10^d` is representable.
      fn max_exp() -> u32 {
        let mut exp = 0;
        while representable(exp + 1) {
          exp += 1;
        }
        exp
      }

      /// Values around zero, the decimal limits and the native limits.
      pub(super) fn samples() -> Vec<$int> {
        core_tests!(@samples $sign $int)
      }

      #[test]
      fn pow10_and_max_magnitude() {
        assert!(!representable(max_exp() + 1));
        for exp in 0..=max_exp() {
          assert_eq!(native::pow10(exp) as u128, 10u128.pow(exp));
          assert_eq!(native::max_magnitude(exp) as u128, 10u128.pow(exp) - 1);
        }
      }

      #[test]
      fn predicates_and_comparison() {
        for a in samples() {
          assert_eq!(native::is_zero(a), a == 0);
          assert_eq!(native::is_positive(a), a > 0);
          core_tests!(@is_negative $sign a);
          for b in samples() {
            assert_eq!(native::le(a, b), a <= b);
          }
        }
      }

      #[test]
      fn checked_arithmetic() {
        for a in samples() {
          for b in samples() {
            assert_eq!(native::checked_add(a, b), a.checked_add(b));
            assert_eq!(native::checked_sub(a, b), a.checked_sub(b));
          }
        }
      }

      #[test]
      fn mul_pow10() {
        for a in samples() {
          for exp in 0..=max_exp() {
            if let Some(product) = a.checked_mul(native::pow10(exp)) {
              assert_eq!(native::mul_pow10(a, exp), product);
            }
          }
        }
      }

      #[test]
      fn decimal_digits() {
        for a in samples() {
          let mut buf = [0xff; DIGIT_BUF_LEN];
          let start = native::write_digits(a, &mut buf);
          let expected = core_tests!(@magnitude $sign a).to_string();
          assert_eq!(&buf[start..], expected.as_bytes());

          if core_tests!(@is_not_negative $sign a) {
            let digits = expected.as_bytes();
            assert_eq!(native::push_digits(native::ZERO, digits, 0, digits.len()), a);
            // Appending to a prefix continues the same number.
            let split = digits.len() / 2;
            let prefix = native::push_digits(native::ZERO, digits, 0, split);
            assert_eq!(native::push_digits(prefix, digits, split, digits.len()), a);
            // Separators between the digits are skipped.
            let mut separated = Vec::new();
            for (i, &digit) in digits.iter().enumerate() {
              if i == split {
                separated.push(b'.');
              }
              separated.extend([digit, b'_']);
            }
            assert_eq!(native::push_digits(native::ZERO, &separated, 0, separated.len()), a);
          }
        }
      }

      #[test]
      fn i128_round_trip() {
        for a in samples() {
          core_tests!(@i128_round_trip $sign a);
        }
        // The largest coefficient that the type holds converts exactly.
        let max = native::max_magnitude(max_exp());
        assert_eq!(native::to_i128(max) as u128, max as u128);
        assert_eq!(native::from_i128(native::to_i128(max)), max);
      }

      core_tests!(@sign_tests $sign $int);
    }
  )+};

  (@samples signed $int:ident) => {{
    let mut values = std::vec![0, 1, -1, 9, -9, 10, -10, <$int>::MAX, <$int>::MIN];
    for exp in 1..=max_exp() {
      let power = native::pow10(exp);
      values.extend([power, power - 1, power + 1, -power, -(power - 1), -(power + 1)]);
    }
    values
  }};
  (@samples unsigned $int:ident) => {{
    let mut values = std::vec![0, 1, 9, 10, <$int>::MAX - 1, <$int>::MAX];
    for exp in 1..=max_exp() {
      let power = native::pow10(exp);
      values.extend([power, power - 1, power + 1]);
    }
    values
  }};

  (@is_negative signed $a:ident) => {
    assert_eq!(native::is_negative($a), $a < 0)
  };
  (@is_negative unsigned $a:ident) => {};

  (@magnitude signed $a:ident) => {
    $a.unsigned_abs()
  };
  (@magnitude unsigned $a:ident) => {
    $a
  };

  (@is_not_negative signed $a:ident) => {
    $a >= 0
  };
  (@is_not_negative unsigned $a:ident) => {
    true
  };

  (@i128_round_trip signed $a:ident) => {
    assert_eq!(native::to_i128($a), $a as i128);
    assert_eq!(native::from_i128(native::to_i128($a)), $a);
  };
  // A `u128` converts exactly only up to `i128::MAX`, which covers every coefficient.
  (@i128_round_trip unsigned $a:ident) => {
    let wide = u128::from($a);
    if wide <= i128::MAX as u128 {
      assert_eq!(native::to_i128($a) as u128, wide);
      assert_eq!(native::from_i128(native::to_i128($a)), $a);
    }
  };

  (@sign_tests signed $int:ident) => {
    #[test]
    fn neg_and_abs() {
      for a in samples().into_iter().filter(|&a| a != <$int>::MIN) {
        assert_eq!(native::neg(a), -a);
        assert_eq!(native::abs(a), a.abs());
      }
    }
  };
  (@sign_tests unsigned $int:ident) => {};
}

core_tests!(
  unsigned u8,
  unsigned u16,
  unsigned u32,
  unsigned u64,
  unsigned u128,
  signed i8,
  signed i16,
  signed i32,
  signed i64,
  signed i128,
);

/// The functions that only storage-generic code uses, which exist only in 128 bits.
mod wide {
  use crate::int;

  #[test]
  fn unsigned_powers_of_ten() {
    for a in super::u128::samples() {
      for exp in 0..=38 {
        let power = 10u128.pow(exp);
        assert_eq!(int::u128::checked_mul_pow10(a, exp), a.checked_mul(power));
        assert_eq!(int::u128::div_rem_pow10(a, exp), (a / power, a % power));
      }
    }
  }

  #[test]
  fn signed_powers_of_ten() {
    for a in super::i128::samples() {
      for exp in 0..=38 {
        let power = 10i128.pow(exp);
        assert_eq!(int::i128::checked_mul_pow10(a, exp), a.checked_mul(power));
        // As with `/` and `%`, the quotient and the remainder have the sign of `a`.
        assert_eq!(int::i128::div_rem_pow10(a, exp), (a / power, a % power));
      }
    }
  }

  #[test]
  fn signed_mul() {
    for a in super::i128::samples() {
      for b in super::i128::samples() {
        if let Some(product) = a.checked_mul(b) {
          assert_eq!(int::i128::mul(a, b), product);
        }
      }
    }
  }
}
