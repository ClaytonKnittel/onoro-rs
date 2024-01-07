import React from 'react';

import { OnoroSocket } from 'client/onoro/socket_msgs';
import { AsyncSocketContext } from 'client/util/async_sockets';
import { GameState } from 'proto/game_state';

export function App() {
  const socket_ref = React.useRef<OnoroSocket>(
    new AsyncSocketContext('ws://[::]:2345/onoro')
  );
  const g: GameState = {
    pawns: [],
    blackTurn: false,
    finished: false,
    turnNum: 0,
  };

  const getGame = async () => {
    const gameRes = await socket_ref.current.call('new_game');
    console.log(gameRes);
  };

  setTimeout(getGame, 1000);

  return <>Hello world</>;
}
