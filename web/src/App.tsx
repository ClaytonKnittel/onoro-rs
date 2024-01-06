import React from 'react';

import { GameState } from 'proto/game_state';

export function App() {
  const g: GameState = {
    pawns: [],
    blackTurn: false,
    finished: false,
    turnNum: 0,
  };
  console.log(g);
  return <>Hello world</>;
}
