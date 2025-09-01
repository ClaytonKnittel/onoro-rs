use crate::{MoveGenerator, OnoroImpl, OnoroView};

pub type Onoro8 = OnoroImpl<8>;
pub type Onoro16 = OnoroImpl<16>;

pub type Onoro8View = OnoroView<8>;
pub type Onoro16View = OnoroView<16>;

pub type Onoro8MoveIterator = MoveGenerator<8>;
pub type Onoro16MoveIterator = MoveGenerator<16>;
