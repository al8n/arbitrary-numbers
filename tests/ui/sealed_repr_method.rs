use numo::UintRepr;

// A bound `R: UintRepr` reaches the methods of its sealed supertrait, which take a token that only
// the crate can make.
fn compare<R: UintRepr>(lhs: &R, rhs: &R) {
  let _ordering = lhs.compare(rhs);
}

fn main() {}
