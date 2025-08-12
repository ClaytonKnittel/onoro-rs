use algebra::{
  direct_product_type,
  group::{Cyclic, Dihedral},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymmetryClassContainer<C, V, E, CV, CE, EV, T> {
  /// Center of mass lies in the center of a hexagonal tile.
  C(C),
  /// Center of mass lies on a vertex of a hexagonal tile.
  V(V),
  /// Center of mass lies on the midpoint of an edge of a hexagonal tile.
  E(E),
  /// Center of mass lies on a line connecting the center of a hexagonal tile to
  /// one of its vertices.
  CV(CV),
  /// Center of mass lies on a line connecting the center of a hexagonal tile to
  /// the midpoint of one if its edges.
  CE(CE),
  /// Center of mass lies on the edge of a hexagonal tile.
  EV(EV),
  /// Center of mass is none of the above.
  Trivial(T),
}

impl<I, C, V, E, CV, CE, EV, T> Iterator for SymmetryClassContainer<C, V, E, CV, CE, EV, T>
where
  C: Iterator<Item = I>,
  V: Iterator<Item = I>,
  E: Iterator<Item = I>,
  CV: Iterator<Item = I>,
  CE: Iterator<Item = I>,
  EV: Iterator<Item = I>,
  T: Iterator<Item = I>,
{
  type Item = I;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      Self::C(val) => val.next(),
      Self::V(val) => val.next(),
      Self::E(val) => val.next(),
      Self::CV(val) => val.next(),
      Self::CE(val) => val.next(),
      Self::EV(val) => val.next(),
      Self::Trivial(val) => val.next(),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymmetryClass {
  /// Center of mass lies in the center of a hexagonal tile.
  C,
  /// Center of mass lies on a vertex of a hexagonal tile.
  V,
  /// Center of mass lies on the midpoint of an edge of a hexagonal tile.
  E,
  /// Center of mass lies on a line connecting the center of a hexagonal tile to
  /// one of its vertices.
  CV,
  /// Center of mass lies on a line connecting the center of a hexagonal tile to
  /// the midpoint of one if its edges.
  CE,
  /// Center of mass lies on the edge of a hexagonal tile.
  EV,
  /// Center of mass is none of the above.
  Trivial,
}

pub type D6 = Dihedral<6>;
pub type D3 = Dihedral<3>;
pub type C2 = Cyclic<2>;
pub type K4 = direct_product_type!(Cyclic<2>, Cyclic<2>);
