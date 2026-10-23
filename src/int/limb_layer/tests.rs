//! The unit tests that every instance of `limb_layer!` shares.
//!
//! `limb_layer_tests!` writes them once, in terms of the instance's `Limb` and its number of bits
//! `B`, into the `tests` module of each instance, so that every test keeps the path of its
//! instance. The inputs depend on the limb type only through `B`: the `N`, the seeds and the order
//! of the random draws are the same for every instance. Each instance adds the tests that compare
//! it with native integers, whose widths differ from one limb type to another.

/// Defines, in the `tests` module of an instance of `limb_layer!`, the tests that every instance
/// shares, their helpers, and the imports that they and the instance's own tests use:
/// `limb_layer_tests!(unsigned)` for a lower layer, and `limb_layer_tests!(signed unsigned: m)`
/// for an upper layer over the lower layer `m`.
macro_rules! limb_layer_tests {
  (unsigned) => {
    use core::cmp::Ordering;
    use std::{string::ToString, vec, vec::Vec};

    use num_bigint::BigInt;

    use super::*;
    use crate::{bigint, int::DIGIT_BUF_LEN};

    /// The number of bits of a limb.
    const B: u32 = Limb::BITS;

    /// The limb with only its top bit set.
    const HIGH: Limb = 1 << (B - 1);

    /// `2^(B * k)`.
    fn boundary<const N: usize>(k: usize) -> [Limb; N] {
      let mut value = [0; N];
      value[k] = 1;
      value
    }

    /// `2^(B * k) - 1`: the `k` lowest limbs all ones.
    fn ones_below<const N: usize>(k: usize) -> [Limb; N] {
      let mut value = [0; N];
      value[..k].fill(Limb::MAX);
      value
    }

    fn carries_and_borrows<const N: usize>() {
      let one = boundary::<N>(0);
      for k in 1..N {
        // A carry and a borrow across the `k` lowest limbs.
        assert_eq!(
          checked_add(&ones_below::<N>(k), &one),
          Some(boundary::<N>(k))
        );
        assert_eq!(
          checked_sub(&boundary::<N>(k), &one),
          Some(ones_below::<N>(k))
        );
        // And across one limb, from limb `k - 1` into limb `k`.
        let mut low = [0; N];
        low[k - 1] = Limb::MAX;
        let mut high = [0; N];
        high[k - 1] = HIGH;
        let mut carried = [0; N];
        carried[k - 1] = HIGH - 1;
        carried[k] = 1;
        assert_eq!(checked_add(&low, &high), Some(carried));
        assert_eq!(checked_sub(&carried, &high), Some(low));
      }
      // Out of the top limb.
      assert_eq!(overflowing_add(&[Limb::MAX; N], &one), ([0; N], true));
      assert_eq!(checked_add(&[Limb::MAX; N], &one), None);
      assert_eq!(checked_add(&boundary::<N>(N - 1), &[Limb::MAX; N]), None);
      assert_eq!(overflowing_sub(&[0; N], &one), ([Limb::MAX; N], true));
      assert_eq!(checked_sub(&[0; N], &one), None);
      assert_eq!(
        checked_sub(&ones_below::<N>(N - 1), &boundary::<N>(N - 1)),
        None
      );
      assert_eq!(checked_sub(&[Limb::MAX; N], &[Limb::MAX; N]), Some([0; N]));
    }

    #[test]
    fn carries_and_borrows_cross_every_limb() {
      carries_and_borrows::<3>();
      carries_and_borrows::<4>();
      carries_and_borrows::<5>();
      carries_and_borrows::<6>();
      carries_and_borrows::<7>();
      carries_and_borrows::<8>();
    }

    fn numeric_order<const N: usize>() {
      // The array order compares limb 0 first and would put 1 above `2^B`.
      let one = boundary::<N>(0);
      let two_pow_b = boundary::<N>(1);
      assert!(one > two_pow_b);
      assert_eq!(cmp(&one, &two_pow_b), Ordering::Less);
      assert!(le(&one, &two_pow_b) && !le(&two_pow_b, &one));
      assert_eq!(
        cmp(&ones_below::<N>(N - 1), &boundary::<N>(N - 1)),
        Ordering::Less
      );
      assert_eq!(cmp(&[Limb::MAX; N], &[Limb::MAX; N]), Ordering::Equal);
    }

    #[test]
    fn comparison_is_numeric() {
      numeric_order::<3>();
      numeric_order::<4>();
      numeric_order::<5>();
      numeric_order::<6>();
      numeric_order::<7>();
      numeric_order::<8>();
    }

    fn maximal_products<const N: usize>() {
      let max = bigint::from_limbs(&[Limb::MAX; N], false);
      let root = max.sqrt();
      let operands = [
        BigInt::ZERO,
        BigInt::from(1u8),
        root.clone(),
        &root + 1u8,
        max.clone(),
        BigInt::from(Limb::MAX),
        bigint::from_limbs(&ones_below::<N>(N - 1), false),
        bigint::from_limbs(&boundary::<N>(N - 1), false),
      ];
      for a in &operands {
        for b in &operands {
          let product = a * b;
          let (Some(limbs_a), Some(limbs_b)) = (
            bigint::to_limbs_of::<Limb, N>(a, false),
            bigint::to_limbs_of(b, false),
          ) else {
            panic!("operands fit");
          };
          match bigint::to_limbs_of::<Limb, N>(&product, false) {
            Some(expected) => assert_eq!(mul(&limbs_a, &limbs_b), expected, "{a} * {b}"),
            // The maximal operands whose product fits are around the square root.
            None => assert!(a > &root || b > &root),
          }
        }
      }
      // The largest square is `root^2`.
      let square = bigint::to_limbs_of::<Limb, N>(&(&root * &root), false).expect("fits");
      let root_limbs = bigint::to_limbs_of::<Limb, N>(&root, false).expect("fits");
      assert_eq!(mul(&root_limbs, &root_limbs), square);
      assert!(bigint::to_limbs_of::<Limb, N>(&((&root + 1u8) * (&root + 1u8)), false).is_none());
    }

    #[test]
    fn multiplication_with_maximal_operands() {
      maximal_products::<3>();
      maximal_products::<4>();
      maximal_products::<5>();
      maximal_products::<6>();
      maximal_products::<7>();
      maximal_products::<8>();
    }

    fn division_by_every_power<const N: usize>() {
      let max = bigint::from_limbs(&[Limb::MAX; N], false);
      for value in [
        max.clone(),
        bigint::pow10(bigint::max_exp(&max)),
        BigInt::from(1u8),
        BigInt::ZERO,
      ] {
        let limbs = bigint::to_limbs_of::<Limb, N>(&value, false).expect("fits");
        // Beyond the largest power of ten that fits, the quotient is zero.
        for exp in 0..=bigint::max_exp(&max) + 2 {
          let power = bigint::pow10(exp);
          let (quotient, remainder) = div_rem_pow10(&limbs, exp);
          assert_eq!(bigint::from_limbs(&quotient, false), &value / &power);
          assert_eq!(bigint::from_limbs(&remainder, false), &value % &power);
        }
      }
    }

    #[test]
    fn div_rem_pow10_for_every_exponent() {
      division_by_every_power::<3>();
      division_by_every_power::<4>();
      division_by_every_power::<5>();
      division_by_every_power::<6>();
      division_by_every_power::<7>();
      division_by_every_power::<8>();
    }

    fn digits_of_the_limits<const N: usize>() {
      for value in [[0; N], [Limb::MAX; N], boundary::<N>(N - 1)] {
        let mut buf = [0xff; DIGIT_BUF_LEN];
        let start = write_digits(&value, &mut buf);
        let expected = bigint::from_limbs(&value, false).to_string();
        assert_eq!(&buf[start..], expected.as_bytes());

        // The digits read back, also with separators among them.
        let digits = expected.as_bytes();
        assert_eq!(push_digits(&[0; N], digits, 0, digits.len()), value);
        let separated: Vec<u8> = digits
          .iter()
          .flat_map(|&digit| [b'_', digit, b'.'])
          .collect();
        assert_eq!(push_digits(&[0; N], &separated, 0, separated.len()), value);
      }
    }

    #[test]
    fn digits_of_zero_and_max() {
      digits_of_the_limits::<3>();
      digits_of_the_limits::<4>();
      digits_of_the_limits::<5>();
      digits_of_the_limits::<6>();
      digits_of_the_limits::<7>();
      digits_of_the_limits::<8>();
      let mut buf = [0; DIGIT_BUF_LEN];
      assert_eq!(
        write_digits(&[Limb::MAX; 512 / B as usize], &mut buf),
        0,
        "u512::MAX fills the buffer"
      );
    }

    #[test]
    fn resizing() {
      let value = [1, 2, 3];
      assert_eq!(resize::<3, 5>(&value), [1, 2, 3, 0, 0]);
      assert_eq!(checked_resize::<3, 5>(&value), Some([1, 2, 3, 0, 0]));
      assert_eq!(resize::<3, 2>(&value), [1, 2]);
      assert_eq!(checked_resize::<3, 2>(&value), None);
      assert_eq!(checked_resize::<3, 2>(&[1, 2, 0]), Some([1, 2]));
      assert_eq!(checked_resize::<5, 3>(&[1, 2, 3, 0, 0]), Some(value));
      assert_eq!(checked_resize::<5, 3>(&[1, 2, 3, 0, 1]), None);
      assert_eq!(
        checked_resize::<3, 3>(&[Limb::MAX; 3]),
        Some([Limb::MAX; 3])
      );
      // Unsigned limbs extend with zeros, whatever the top bit.
      assert_eq!(resize::<2, 3>(&[0, HIGH]), [0, HIGH, 0]);
    }

    /// A deterministic xorshift generator, so that a failure replays from the seed.
    struct Rng(u64);

    impl Rng {
      fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
      }

      /// A nonzero value with a random number of significant limbs, each random, zero or all
      /// ones, and a top limb of random bit length, which gives every normalization shift.
      fn limbs<const N: usize>(&mut self) -> [Limb; N] {
        let mut value = [0; N];
        let count = (self.next() % N as u64) as usize + 1;
        for limb in &mut value[..count] {
          *limb = match self.next() % 4 {
            0 => 0,
            1 => Limb::MAX,
            _ => self.next() as Limb,
          };
        }
        value[count - 1] = (value[count - 1] >> (self.next() % u64::from(B))).max(1);
        value
      }
    }

    /// Checks `div_rem(a, b)` against `num-bigint`.
    fn check_division<const N: usize>(a: &[Limb; N], b: &[Limb; N]) {
      let (quotient, remainder) = div_rem(a, b);
      let (big_a, big_b) = (bigint::from_limbs(a, false), bigint::from_limbs(b, false));
      assert_eq!(
        bigint::from_limbs(&quotient, false),
        &big_a / &big_b,
        "{a:x?} / {b:x?}"
      );
      assert_eq!(
        bigint::from_limbs(&remainder, false),
        &big_a % &big_b,
        "{a:x?} % {b:x?}"
      );
    }

    fn division_against_big_integers<const N: usize>() {
      // Structured operands: limb boundaries, all-ones runs, single limbs, and a top limb with
      // the high bit set or with only the low bit.
      let mut operands = vec![[0; N], boundary::<N>(0), [Limb::MAX; N]];
      for k in 1..N {
        operands.extend([boundary::<N>(k), ones_below::<N>(k)]);
        let mut top = ones_below::<N>(k);
        top[k] = HIGH;
        operands.push(top);
      }
      let mut seven = [0; N];
      seven[0] = 7;
      operands.push(seven);
      let operands = bigint::thin(operands);
      for a in &operands {
        for b in operands.iter().filter(|b| !is_zero(b)) {
          check_division(a, b);
        }
      }
      let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ N as u64);
      let count = if cfg!(miri) { 8 } else { 3000 };
      for _ in 0..count {
        let b = rng.limbs::<N>();
        let a = match rng.next() % 4 {
          0 => [Limb::MAX; N],
          _ => rng.limbs::<N>(),
        };
        check_division(&a, &b);
      }
    }

    #[test]
    fn division_against_big_integers_for_every_width() {
      division_against_big_integers::<1>();
      division_against_big_integers::<2>();
      division_against_big_integers::<3>();
      division_against_big_integers::<4>();
      division_against_big_integers::<5>();
      division_against_big_integers::<6>();
      division_against_big_integers::<7>();
      division_against_big_integers::<8>();
    }

    /// The add-back step of Algorithm D, each reached exactly once and past the shortcut for a
    /// dividend below the divisor. Both divisors have three significant limbs: with two, the
    /// correction by the next divisor limb is exact and nothing is ever added back.
    #[test]
    fn division_adds_back() {
      const MAX: Limb = Limb::MAX;
      // Two steps with shift 0. The second estimates 8, which passes the test with the second
      // divisor limb, zero, but the third divisor limb makes the subtraction borrow.
      let a = [MAX - 9, 7, 0, 4];
      let b = [MAX, 0, HIGH, 0];
      assert_eq!(div_rem(&a, &b), ([7, 0, 0, 0], [MAX - 2, 0, HIGH, 0]));
      check_division(&a, &b);
      // A divisor as wide as the dividend: one step, with shift `B - 1`, which estimates 2.
      let a = [MAX - 2, 1, 2];
      let b = [MAX, 0, 1];
      assert_eq!(div_rem(&a, &b), ([1, 0, 0], [MAX - 1, 0, 1]));
      check_division(&a, &b);
    }

    /// Steps whose window has the top divisor limb on top, so that the quotient of the window's
    /// top two limbs by that limb is `2^B` or `2^B + 1`, and the estimate starts at `2^B - 1`.
    fn division_with_equal_top_limbs() {
      const MAX: Limb = Limb::MAX;
      // The first step estimates 1 and corrects it to 0, leaving the top limb `HIGH`: in the
      // second step, the quotient of the top two limbs is `2^B`.
      let a = [0, 0, HIGH];
      let b = [MAX, HIGH, 0];
      assert_eq!(div_rem(&a, &b), ([MAX - 1, 0, 0], [MAX - 1, 2, 0]));
      check_division(&a, &b);
      // The same with `HIGH + 5` below the top limb: the quotient is `2^B + 1`.
      let a = [7, HIGH + 5, HIGH];
      assert_eq!(div_rem(&a, &b), ([MAX, 0, 0], [6, 7, 0]));
      check_division(&a, &b);
    }

    /// Divisors whose top limb has 0, 1, `B / 2 - 1`, `B / 2`, `B - 2` and `B - 1` leading
    /// zeros, including the shift of zero, which shifts by nothing.
    #[test]
    fn division_for_every_normalization_shift() {
      for shift in [0, 1, B / 2 - 1, B / 2, B - 2, B - 1] {
        let top = Limb::MAX >> shift;
        let b = [Limb::MAX, 0, top];
        for a in [
          [Limb::MAX; 3],
          b,
          [0, 0, top],
          [1, 0, top],
          [Limb::MAX, Limb::MAX, top],
        ] {
          check_division(&a, &b);
        }
        let mut wide = [Limb::MAX; 8];
        wide[7] = top;
        check_division(&[Limb::MAX; 8], &wide);
        check_division(&[Limb::MAX; 8], &[Limb::MAX, top, 0, 0, 0, 0, 0, 0]);
      }
    }

    fn division_edges<const N: usize>() {
      let one = boundary::<N>(0);
      let max = [Limb::MAX; N];
      assert_eq!(div_rem(&max, &one), (max, [0; N]));
      assert_eq!(div_rem(&max, &max), (one, [0; N]));
      assert_eq!(div_rem(&[0; N], &max), ([0; N], [0; N]));
      // A dividend below the divisor, and equal to it, for a divisor as wide as the dividend.
      let mut divisor = ones_below::<N>(N - 1);
      divisor[N - 1] = 1;
      let mut below = divisor;
      below[0] -= 1;
      assert_eq!(div_rem(&below, &divisor), ([0; N], below));
      assert_eq!(div_rem(&divisor, &divisor), (one, [0; N]));
      // A divisor with leading zero limbs and one significant limb takes the single-limb
      // division.
      let mut seven = [0; N];
      seven[0] = 7;
      check_division(&max, &seven);
      // A divisor as wide as the dividend leaves one quotient limb.
      let mut wide = max;
      wide[N - 1] = 1;
      check_division(&max, &wide);
    }

    #[test]
    fn division_edges_for_every_width() {
      division_edges::<2>();
      division_edges::<3>();
      division_edges::<4>();
      division_edges::<5>();
      division_edges::<6>();
      division_edges::<7>();
      division_edges::<8>();
    }

    /// Checks `round_div(value, divisor, mode)` against rounding by distance.
    fn check_rounding<const N: usize>(value: &[Limb; N], divisor: &[Limb; N]) {
      let (big_value, big_divisor) = (
        bigint::from_limbs(value, false),
        bigint::from_limbs(divisor, false),
      );
      let truncated = &big_value / &big_divisor;
      for mode in bigint::MODES {
        let (rounded, remainder, away) = round_div(value, divisor, mode);
        let expected = bigint::round(&big_value, &big_divisor, mode);
        let rounded = bigint::from_limbs(&rounded, false);
        assert_eq!(rounded, expected, "{value:x?} / {divisor:x?}, {mode:?}");
        assert_eq!(
          bigint::from_limbs(&remainder, false),
          &big_value % &big_divisor
        );
        assert_eq!(away, rounded != truncated);
      }
    }

    fn rounding_division<const N: usize>() {
      let mut rng = Rng(0x5eed_f00d ^ N as u64);
      let count = if cfg!(miri) { 4 } else { 400 };
      for _ in 0..count {
        let divisor = rng.limbs::<N>();
        // Values at and around the multiples of the divisor and the halfway points between
        // them.
        let big_divisor = bigint::from_limbs(&divisor, false);
        let quotient = bigint::from_limbs(&rng.limbs::<N>(), false) >> (B * N as u32 / 2);
        let base = &quotient * &big_divisor;
        let half = &big_divisor / 2u8;
        for offset in [
          BigInt::ZERO,
          BigInt::from(1u8),
          &half - 1u8,
          half.clone(),
          &half + 1u8,
          &big_divisor - 1u8,
        ] {
          if let Some(value) = bigint::to_limbs_of::<Limb, N>(&(&base + offset), false) {
            check_rounding(&value, &divisor);
          }
        }
        check_rounding(&rng.limbs::<N>(), &divisor);
      }
      // Even divisors, whose halfway points are ties, with an even and an odd quotient.
      let mut ten = [0; N];
      ten[0] = 10;
      for value in [5, 15, 25, 4, 6, 14, 16] {
        let mut limbs = [0; N];
        limbs[0] = value;
        check_rounding(&limbs, &ten);
      }
      check_rounding(&[Limb::MAX; N], &boundary::<N>(N - 1));
    }

    #[test]
    fn rounding_division_matches_rounding_by_distance() {
      rounding_division::<1>();
      rounding_division::<2>();
      rounding_division::<3>();
      rounding_division::<5>();
      rounding_division::<8>();
    }

    /// The widths at which the functions of a width are checked: 1, `B - 1`, `B`, `B + 1`,
    /// `B * N - 1` and `B * N`, those that `N` limbs hold.
    fn widths<const N: usize>() -> Vec<u32> {
      let top = B * N as u32;
      let mut widths: Vec<u32> = [1, B - 1, B, B + 1, top - 1, top]
        .into_iter()
        .filter(|&width| width <= top)
        .collect();
      widths.sort_unstable();
      widths.dedup();
      widths
    }

    fn widths_against_big_integers<const N: usize>() {
      let widths = widths::<N>();
      let patterns = bigint::bit_patterns_of::<Limb, N>(&widths);
      let values: Vec<BigInt> = patterns
        .iter()
        .map(|pattern| bigint::from_limbs(pattern, false))
        .collect();
      for &bits in &widths {
        let max = bigint::max(bits, false);
        assert_eq!(bigint::from_limbs(&max_of_bits::<N>(bits), false), max);
        for (pattern, value) in patterns.iter().zip(&values) {
          assert_eq!(
            fits_bits(pattern, bits),
            *value <= max,
            "{pattern:x?} in {bits} bits"
          );
        }
      }
      assert_eq!(max_of_bits::<N>(0), [0; N]);
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn widths_match_big_integers() {
      widths_against_big_integers::<1>();
      widths_against_big_integers::<2>();
      widths_against_big_integers::<3>();
      widths_against_big_integers::<4>();
      widths_against_big_integers::<8>();
      widths_against_big_integers::<16>();
      widths_against_big_integers::<64>();
    }

    fn digits_into_a_slice<const N: usize>() {
      let mut patterns = vec![[0; N], [Limb::MAX; N]];
      // Each limb boundary, or under Miri, which divides many limbs slowly, only the top one.
      let boundaries = if cfg!(miri) { N - 1..N } else { 0..N };
      for k in boundaries {
        patterns.extend([boundary::<N>(k), ones_below::<N>(k + 1)]);
      }
      for pattern in patterns {
        let expected = bigint::from_limbs(&pattern, false).to_string();
        // `(MAX_STEP + 1) * N` bytes hold every value, and so do exactly as many bytes as there
        // are digits.
        let mut buf = vec![0xff; (MAX_STEP as usize + 1) * N];
        let start = write_digits_into(&pattern, &mut buf);
        assert_eq!(&buf[start..], expected.as_bytes());
        let mut exact = vec![0xff; expected.len()];
        assert_eq!(write_digits_into(&pattern, &mut exact), 0);
        assert_eq!(exact, expected.as_bytes());
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn digits_into_a_slice_for_every_width() {
      digits_into_a_slice::<1>();
      digits_into_a_slice::<2>();
      digits_into_a_slice::<3>();
      digits_into_a_slice::<4>();
      digits_into_a_slice::<8>();
      digits_into_a_slice::<16>();
      digits_into_a_slice::<64>();
      // `2^4096 - 1` has 1234 digits.
      let mut buf = [0; 1234];
      assert_eq!(
        write_digits_into(&[Limb::MAX; 4096 / B as usize], &mut buf),
        0
      );
      assert_eq!(
        &buf[..],
        bigint::from_limbs(&[Limb::MAX; 4096 / B as usize], false)
          .to_string()
          .as_bytes()
      );
    }
  };
  (signed unsigned: $unsigned:ident) => {
    use core::cmp::Ordering;
    use std::{string::ToString, vec, vec::Vec};

    use num_bigint::BigInt;

    use super::*;
    use crate::{bigint, int::DIGIT_BUF_LEN};

    /// The number of bits of a limb.
    const B: u32 = Limb::BITS;

    /// The limb with only its top bit set.
    const HIGH: Limb = 1 << (B - 1);

    /// `MIN`, with only the sign bit set.
    fn min<const N: usize>() -> [Limb; N] {
      let mut min = [0; N];
      min[N - 1] = HIGH;
      min
    }

    /// `MAX`, with every bit but the sign bit set.
    fn max<const N: usize>() -> [Limb; N] {
      let mut max = [Limb::MAX; N];
      max[N - 1] = HIGH - 1;
      max
    }

    fn sign_transitions<const N: usize>() {
      let one = {
        let mut one = [0; N];
        one[0] = 1;
        one
      };
      let minus_one = [Limb::MAX; N];
      // The top limb's sign bit flips exactly at `MAX + 1` and `MIN - 1`.
      assert!(!is_negative(&max::<N>()) && is_negative(&min::<N>()));
      assert_eq!(checked_add(&max::<N>(), &one), None);
      assert_eq!(checked_sub(&min::<N>(), &one), None);
      assert_eq!(checked_add(&min::<N>(), &minus_one), None);
      assert_eq!(checked_sub(&max::<N>(), &minus_one), None);
      assert_eq!(checked_add(&minus_one, &one), Some([0; N]));
      assert_eq!(checked_add(&max::<N>(), &min::<N>()), Some(minus_one));
      assert_eq!(checked_sub(&min::<N>(), &min::<N>()), Some([0; N]));
      assert_eq!(
        checked_sub(&[0; N], &max::<N>()),
        Some({
          let mut negated = min::<N>();
          negated[0] = 1;
          negated
        })
      );
      assert_eq!(checked_sub(&[0; N], &min::<N>()), None);

      // Numeric order, where the unsigned order of the limbs puts the negative values on top.
      assert_eq!(cmp(&min::<N>(), &minus_one), Ordering::Less);
      assert_eq!(cmp(&minus_one, &[0; N]), Ordering::Less);
      assert_eq!(cmp(&[0; N], &one), Ordering::Less);
      assert_eq!(cmp(&one, &max::<N>()), Ordering::Less);
      assert_eq!(cmp(&minus_one, &max::<N>()), Ordering::Less);
      assert_eq!(
        super::$unsigned::cmp(&minus_one, &max::<N>()),
        Ordering::Greater
      );

      // `MIN` has no negation, but its magnitude is unsigned.
      assert_eq!(unsigned_abs(&min::<N>()), min::<N>());
      assert_eq!(neg(&max::<N>()), {
        let mut negated = min::<N>();
        negated[0] = 1;
        negated
      });
      assert_eq!(abs(&minus_one), one);

      // Scaling reaches the limits on both sides, and one step beyond them fails; the magnitude
      // of `MIN` itself scales by `10^0`.
      assert_eq!(checked_mul_pow10(&min::<N>(), 0), Some(min::<N>()));
      assert_eq!(mul_pow10(&min::<N>(), 0), min::<N>());
      for limit in [min::<N>(), max::<N>()] {
        let limit = bigint::from_limbs(&limit, true);
        let inside = &limit / 10u8;
        let outside = if limit < BigInt::ZERO {
          &inside - 1u8
        } else {
          &inside + 1u8
        };
        let scaled = bigint::to_limbs_of::<Limb, N>(&(&inside * 10u8), true);
        assert!(scaled.is_some());
        let inside = bigint::to_limbs_of::<Limb, N>(&inside, true).expect("fits");
        let outside = bigint::to_limbs_of::<Limb, N>(&outside, true).expect("fits");
        assert_eq!(checked_mul_pow10(&inside, 1), scaled);
        assert_eq!(checked_mul_pow10(&outside, 1), None);
      }
    }

    #[test]
    fn sign_transitions_of_the_top_limb() {
      sign_transitions::<3>();
      sign_transitions::<4>();
      sign_transitions::<5>();
      sign_transitions::<6>();
      sign_transitions::<7>();
      sign_transitions::<8>();
    }

    fn digits_of_min_and_max<const N: usize>() {
      for value in [min::<N>(), max::<N>(), [Limb::MAX; N], [0; N]] {
        let mut buf = [0xff; DIGIT_BUF_LEN];
        let start = write_digits(&value, &mut buf);
        let expected = bigint::from_limbs(&value, true).magnitude().to_string();
        assert_eq!(&buf[start..], expected.as_bytes());
      }
    }

    #[test]
    fn digits_of_zero_min_and_max() {
      digits_of_min_and_max::<3>();
      digits_of_min_and_max::<4>();
      digits_of_min_and_max::<5>();
      digits_of_min_and_max::<6>();
      digits_of_min_and_max::<7>();
      digits_of_min_and_max::<8>();
      let mut buf = [0; DIGIT_BUF_LEN];
      assert_eq!(
        write_digits(&min::<{ 512 / B as usize }>(), &mut buf),
        1,
        "|i512::MIN| has one digit fewer than the buffer"
      );
    }

    /// Every conversion between two and three limbs of either sign, against `num-bigint`.
    #[test]
    fn conversions_between_signs_and_widths() {
      let mut values: Vec<BigInt> = vec![BigInt::ZERO, BigInt::from(1u8), BigInt::from(-1)];
      for bits in [B - 1, B, 2 * B - 1, 2 * B, 3 * B - 1, 3 * B] {
        let power = BigInt::from(1u8) << bits;
        values.extend([&power - 1u8, power.clone(), &power + 1u8]);
        values.extend([1u8 - &power, -&power, -&power - 1u8]);
      }
      for value in &values {
        let two_signed = bigint::to_limbs_of::<Limb, 2>(value, true);
        let two_unsigned = bigint::to_limbs_of::<Limb, 2>(value, false);
        let three_signed = bigint::to_limbs_of::<Limb, 3>(value, true);
        let three_unsigned = bigint::to_limbs_of::<Limb, 3>(value, false);

        if let Some(limbs) = two_signed {
          assert_eq!(checked_resize::<2, 3>(&limbs), three_signed);
          assert_eq!(resize::<2, 3>(&limbs), three_signed.expect("wider"));
          assert_eq!(checked_to_unsigned::<2, 3>(&limbs), three_unsigned);
          assert_eq!(checked_to_unsigned::<2, 2>(&limbs), two_unsigned);
        }
        if let Some(limbs) = three_signed {
          assert_eq!(checked_resize::<3, 2>(&limbs), two_signed);
          assert_eq!(checked_to_unsigned::<3, 2>(&limbs), two_unsigned);
          if let Some(narrow) = two_signed {
            assert_eq!(resize::<3, 2>(&limbs), narrow);
          }
        }
        if let Some(limbs) = two_unsigned {
          assert_eq!(checked_from_unsigned::<2, 3>(&limbs), three_signed);
          assert_eq!(checked_from_unsigned::<2, 2>(&limbs), two_signed);
        }
        if let Some(limbs) = three_unsigned {
          assert_eq!(checked_from_unsigned::<3, 2>(&limbs), two_signed);
          assert_eq!(checked_from_unsigned::<3, 3>(&limbs), three_signed);
        }
      }
    }

    /// The value `value`, which must fit `N` limbs.
    fn limbs<const N: usize>(value: i64) -> [Limb; N] {
      bigint::to_limbs_of(&BigInt::from(value), true).expect("fits")
    }

    fn division_with_min<const N: usize>() {
      let min = min::<N>();
      let one = limbs::<N>(1);
      let mut half_min = [0; N];
      half_min[N - 1] = HIGH | HIGH >> 1;
      assert_eq!(div_rem(&min, &one), (min, [0; N]));
      assert_eq!(div_rem(&min, &limbs(2)), (half_min, [0; N]));
      assert_eq!(div_rem(&min, &min), (one, [0; N]));
      // `2^(B * N - 1) = 2 (mod 3)`, an odd power of two, so the remainder is `-2`.
      let (quotient, remainder) = div_rem(&min, &limbs(3));
      assert_eq!(remainder, limbs(-2));
      assert_eq!(
        bigint::from_limbs(&quotient, true),
        bigint::min(B * N as u32, true) / 3
      );
      let min_plus_one =
        bigint::to_limbs_of(&(bigint::min(B * N as u32, true) + 1u8), true).expect("fits");
      assert_eq!(div_rem(&min_plus_one, &limbs(-1)), (max::<N>(), [0; N]));
      assert_eq!(div_rem(&limbs::<N>(-7), &limbs(2)), (limbs(-3), limbs(-1)));
      assert_eq!(div_rem(&limbs::<N>(7), &limbs(-2)), (limbs(-3), limbs(1)));
      assert_eq!(div_rem(&limbs::<N>(-7), &limbs(-2)), (limbs(3), limbs(-1)));
    }

    #[test]
    fn division_with_min_for_every_width() {
      division_with_min::<1>();
      division_with_min::<2>();
      division_with_min::<3>();
      division_with_min::<4>();
      division_with_min::<5>();
      division_with_min::<6>();
      division_with_min::<7>();
      division_with_min::<8>();
    }

    /// A deterministic xorshift generator, so that a failure replays from the seed.
    struct Rng(u64);

    impl Rng {
      fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
      }

      /// A value of either sign whose magnitude has a random number of significant limbs.
      fn signed<const N: usize>(&mut self) -> [Limb; N] {
        let mut magnitude = [0; N];
        let count = (self.next() % N as u64) as usize + 1;
        for limb in &mut magnitude[..count] {
          *limb = self.next() as Limb;
        }
        magnitude[count - 1] >>= self.next() % u64::from(B);
        magnitude[N - 1] &= Limb::MAX >> 1;
        if self.next().is_multiple_of(2) {
          magnitude
        } else {
          neg(&magnitude)
        }
      }
    }

    fn division_against_big_integers<const N: usize>() {
      let mut rng = Rng(0x0dd_ba11 ^ N as u64);
      let count = if cfg!(miri) { 8 } else { 2000 };
      for _ in 0..count {
        let (a, b) = (rng.signed::<N>(), rng.signed::<N>());
        if is_zero(&b) {
          continue;
        }
        let (quotient, remainder) = div_rem(&a, &b);
        let (big_a, big_b) = (bigint::from_limbs(&a, true), bigint::from_limbs(&b, true));
        // `num-bigint` rounds toward zero, as `/` and `%` do.
        assert_eq!(bigint::from_limbs(&quotient, true), &big_a / &big_b);
        assert_eq!(bigint::from_limbs(&remainder, true), &big_a % &big_b);
      }
    }

    #[test]
    fn division_against_big_integers_for_every_width() {
      division_against_big_integers::<3>();
      division_against_big_integers::<4>();
      division_against_big_integers::<8>();
    }

    /// Checks `round_div(value, divisor, mode)` against rounding by distance.
    fn check_rounding<const N: usize>(value: &[Limb; N], divisor: &[Limb; N]) {
      let (big_value, big_divisor) = (
        bigint::from_limbs(value, true),
        bigint::from_limbs(divisor, true),
      );
      let truncated = &big_value / &big_divisor;
      for mode in bigint::MODES {
        let (rounded, remainder, away) = round_div(value, divisor, mode);
        let rounded = bigint::from_limbs(&rounded, true);
        assert_eq!(
          rounded,
          bigint::round(&big_value, &big_divisor, mode),
          "{big_value} / {big_divisor}, {mode:?}"
        );
        assert_eq!(
          bigint::from_limbs(&remainder, true),
          &big_value % &big_divisor
        );
        assert_eq!(away, rounded != truncated);
      }
    }

    fn rounding_division<const N: usize>() {
      // Values below the divisor in magnitude, whose quotient is zero, and ties of each parity.
      for value in [-9, -6, -5, -4, -1, 0, 1, 4, 5, 6, 9, -15, -25, 15, 25] {
        check_rounding(&limbs::<N>(value), &limbs(10));
      }
      check_rounding(&min::<N>(), &limbs(3));
      check_rounding(&max::<N>(), &limbs(2));
      let mut rng = Rng(0xfee1_600d ^ N as u64);
      let count = if cfg!(miri) { 4 } else { 600 };
      for _ in 0..count {
        let divisor = unsigned_abs(&rng.signed::<N>());
        if is_zero(&divisor) {
          continue;
        }
        check_rounding(&rng.signed::<N>(), &divisor);
      }
    }

    #[test]
    fn rounding_division_matches_rounding_by_distance() {
      rounding_division::<1>();
      rounding_division::<2>();
      rounding_division::<3>();
      rounding_division::<8>();
    }

    /// The widths at which the functions of a width are checked: 1, `B - 1`, `B`, `B + 1`,
    /// `B * N - 1` and `B * N`, those that `N` limbs hold.
    fn widths<const N: usize>() -> Vec<u32> {
      let top = B * N as u32;
      let mut widths: Vec<u32> = [1, B - 1, B, B + 1, top - 1, top]
        .into_iter()
        .filter(|&width| width <= top)
        .collect();
      widths.sort_unstable();
      widths.dedup();
      widths
    }

    fn widths_against_big_integers<const N: usize>() {
      let widths = widths::<N>();
      let patterns = bigint::bit_patterns_of::<Limb, N>(&widths);
      let values: Vec<(BigInt, BigInt)> = patterns
        .iter()
        .map(|pattern| {
          (
            bigint::from_limbs(pattern, true),
            bigint::from_limbs(pattern, false),
          )
        })
        .collect();
      for &bits in &widths {
        let (min, max) = (bigint::min(bits, true), bigint::max(bits, true));
        assert_eq!(bigint::from_limbs(&max_of_bits::<N>(bits), true), max);
        assert_eq!(bigint::from_limbs(&min_of_bits::<N>(bits), true), min);
        let modulus = BigInt::from(1u8) << bits;
        for (pattern, (value, unsigned)) in patterns.iter().zip(&values) {
          assert_eq!(
            fits_bits(pattern, bits),
            min <= *value && *value <= max,
            "{pattern:x?} in {bits} bits"
          );
          // The two's complement integer in the low `bits` bits.
          let low = unsigned % &modulus;
          let expected = if low > max { low - &modulus } else { low };
          assert_eq!(
            bigint::from_limbs(&sign_extend(pattern, bits), true),
            expected,
            "{pattern:x?} from {bits} bits"
          );
        }
      }
    }

    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn widths_match_big_integers() {
      widths_against_big_integers::<1>();
      widths_against_big_integers::<2>();
      widths_against_big_integers::<3>();
      widths_against_big_integers::<4>();
      widths_against_big_integers::<8>();
      widths_against_big_integers::<16>();
      widths_against_big_integers::<64>();
    }

    /// The magnitude of `-2^4095`, the smallest value of 4096 bits, has 1233 digits.
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn digits_of_the_smallest_value_of_64_limbs() {
      let min = min_of_bits::<{ 4096 / B as usize }>(4096);
      let mut buf = [0; 1233];
      assert_eq!(
        crate::int::$unsigned::write_digits_into(&unsigned_abs(&min), &mut buf),
        0
      );
      assert_eq!(
        &buf[..],
        (-bigint::from_limbs(&min, true)).to_string().as_bytes()
      );
    }
  };
}

pub(crate) use limb_layer_tests;
