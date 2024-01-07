import React from 'react';

import { GameState } from 'proto/game_state';

export function OnoroBoard(props: { gameState: GameState }) {
  return (
    <>
      <div>
        {props.gameState.blackTurn ?? false ? 'Black turn' : 'White turn'}
      </div>
      <div>Turn {(props.gameState.turnNum ?? 0) + 1}</div>
      {props.gameState.finished ?? false ? <div>Game over!</div> : <></>}
      {props.gameState.pawns.map((pawn) => (
        <div>
          ({pawn.x}, {pawn.y}) {pawn.black ?? false ? 'black' : 'white'}
        </div>
      ))}
    </>
  );
}
