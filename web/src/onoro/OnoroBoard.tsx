import React from 'react';

import Circle from 'client/components/Circle';
import HexGrid from 'client/components/HexGrid';
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
        <div key={`${pawn.x},${pawn.y}`}>
          ({pawn.x}, {pawn.y}) {pawn.black ?? false ? 'black' : 'white'}
        </div>
      ))}
      <HexGrid
        tiles={[
          {
            x: 0,
            y: 0,
            component: (
              <Circle style={{ backgroundColor: 'red' }} radius='25px' />
            ),
          },
          {
            x: 3,
            y: 0,
            component: (
              <Circle style={{ backgroundColor: 'red' }} radius='25px' />
            ),
          },
          {
            x: 2,
            y: 1,
            component: (
              <Circle style={{ backgroundColor: 'red' }} radius='25px' />
            ),
          },
        ]}
        tileSpacing={'100px'}
      />
    </>
  );
}
