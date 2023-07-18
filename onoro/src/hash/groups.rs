use algebra::{
  direct_product_type,
  group::{Cyclic, Dihedral},
};

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
  TRIVIAL,
}

pub type D6 = Dihedral<6>;
pub type D3 = Dihedral<3>;
pub type C2 = Cyclic<2>;
pub type K4 = direct_product_type!(Cyclic<2>, Cyclic<2>);
