macro_rules! core_tests {
  ($($int:ident),+ $(,)?) => {$(
    mod $int {
      use std::string::ToString;

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
      fn samples() -> std::vec::Vec<$int> {
        let mut values = std::vec![0, 1, -1, 9, -9, 10, -10, <$int>::MAX, <$int>::MIN];
        for exp in 1..=max_exp() {
          let power = native::pow10(exp);
          values.extend([power, power - 1, power + 1, -power, -(power - 1), -(power + 1)]);
        }
        values
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
      fn sign_predicates_and_comparison() {
        for a in samples() {
          assert_eq!(native::is_zero(a), a == 0);
          assert_eq!(native::is_negative(a), a < 0);
          assert_eq!(native::is_positive(a), a > 0);
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
            if let Some(product) = a.checked_mul(b) {
              assert_eq!(native::mul(a, b), product);
            }
          }
        }
      }

      #[test]
      fn neg_and_abs() {
        for a in samples().into_iter().filter(|&a| a != <$int>::MIN) {
          assert_eq!(native::neg(a), -a);
          assert_eq!(native::abs(a), a.abs());
        }
      }

      #[test]
      fn powers_of_ten() {
        for a in samples() {
          for exp in 0..=max_exp() {
            let power = 10i128.pow(exp);
            let expected = (a as i128).checked_mul(power).filter(|&p| {
              p >= <$int>::MIN as i128 && p <= <$int>::MAX as i128
            });
            assert_eq!(native::checked_mul_pow10(a, exp).map(|p| p as i128), expected);
            if let Some(product) = expected {
              assert_eq!(native::mul_pow10(a, exp) as i128, product);
            }
            let (quotient, remainder) = native::div_rem_pow10(a, exp);
            assert_eq!(quotient as i128, a as i128 / power);
            assert_eq!(remainder as i128, a as i128 % power);
          }
        }
      }

      #[test]
      fn decimal_digits() {
        for a in samples() {
          let mut buf = [0xff; DIGIT_BUF_LEN];
          let start = native::write_digits(a, &mut buf);
          let expected = a.unsigned_abs().to_string();
          assert_eq!(&buf[start..], expected.as_bytes());

          if a >= 0 {
            let digits = expected.as_bytes();
            assert_eq!(native::push_digits(native::ZERO, digits, 0, digits.len()), a);
            // Appending to a prefix continues the same number.
            let split = digits.len() / 2;
            let prefix = native::push_digits(native::ZERO, digits, 0, split);
            assert_eq!(native::push_digits(prefix, digits, split, digits.len()), a);
          }
        }
      }

      #[test]
      fn i128_round_trip() {
        for a in samples() {
          assert_eq!(native::to_i128(a), a as i128);
          assert_eq!(native::from_i128(native::to_i128(a)), a);
        }
      }
    }
  )+};
}

core_tests!(i8, i16, i32, i64, i128);
