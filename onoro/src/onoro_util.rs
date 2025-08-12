use crate::{error::OnoroError, OnoroIndex};

pub(crate) struct BoardLayoutPawns<Index: OnoroIndex> {
  pub black_pawns: Vec<Index>,
  pub white_pawns: Vec<Index>,
}

pub(crate) fn pawns_from_board_string<Index: OnoroIndex>(
  board_layout: &str,
  n: usize,
) -> Result<BoardLayoutPawns<Index>, OnoroError> {
  let mut black_pawns = Vec::new();
  let mut white_pawns = Vec::new();

  for (y, line) in board_layout.split('\n').enumerate() {
    for (x, tile) in line.split_ascii_whitespace().enumerate() {
      let pos = Index::from_coords(x as u32 + 1, (n - y - 2) as u32);
      match tile {
        "B" | "b" => black_pawns.push(pos),
        "W" | "w" => white_pawns.push(pos),
        "." => {}
        _ => {
          return Err(OnoroError::new(format!(
            "Invalid character in game state string: {tile}"
          )));
        }
      }
    }
  }

  if black_pawns.len() > n || white_pawns.len() > n {
    return Err(OnoroError::new(format!(
      "Too many pawns in board: {} black and {} white",
      black_pawns.len(),
      white_pawns.len()
    )));
  }

  if black_pawns.is_empty() {
    return Err(OnoroError::new(
      "Must have at least one black pawn placed, since they are the first player.".into(),
    ));
  }

  if !((black_pawns.len() - 1)..=black_pawns.len()).contains(&white_pawns.len()) {
    return Err(OnoroError::new(format!(
        "There must be either one fewer or equally many white pawns as there are black. Found {} black and {} white.",
        black_pawns.len(), white_pawns.len()
      )));
  }

  Ok(BoardLayoutPawns {
    black_pawns,
    white_pawns,
  })
}
