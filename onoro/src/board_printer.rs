use std::{collections::HashMap, fmt::Display};

use crate::hex_pos::HexPosOffset;

struct BoardPrinter {
  pieces: HashMap<HexPosOffset, char>,
}

impl Display for BoardPrinter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let ((min_x, min_y), (max_x, max_y)) = self.pieces.keys().fold(
      ((isize::MAX, isize::MAX), (0, 0)),
      |((min_x, min_y), (max_x, max_y)), pos| {
        (
          (min_x.min(pos.x() as isize), min_y.min(pos.y() as isize)),
          (max_x.max(pos.x() as isize), max_y.max(pos.y() as isize)),
        )
      },
    );

    let min_x = min_x - 1;
    let min_y = min_y - 1;
    let max_x = max_x + 1;
    let max_y = max_y + 1;

    for y in (min_y..=max_y).rev() {
      write!(f, "{: <width$}", "", width = (max_y - y) as usize)?;
      for x in min_x..=max_x {
        write!(
          f,
          "{}",
          match self.pieces.get(&HexPosOffset::new(x as i32, y as i32)) {
            Some(&c) => c,
            None => '.',
          }
        )?;

        if x < max_x {
          write!(f, " ")?;
        }
      }

      if y > min_y {
        writeln!(f)?;
      }
    }

    Ok(())
  }
}

pub fn display_on_hexagonal_grid(pieces: HashMap<HexPosOffset, char>) -> impl Display {
  BoardPrinter { pieces }
}
