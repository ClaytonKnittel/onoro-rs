import React from 'react';

import { OnoroBoard } from 'client/onoro/OnoroBoard';
import { OnoroSocket } from 'client/onoro/socket_msgs';
import { AsyncSocketContext } from 'client/util/async_sockets';
import { isOk } from 'client/util/status';
import { GameState } from 'proto/game_state';

const socket: OnoroSocket = new AsyncSocketContext(
  'ws://[::]:2345/onoro',
  true
);

export function App() {
  const [game, setGame] = React.useState<GameState | null>(null);
  const setGameRef = React.useRef(setGame);
  setGameRef.current = setGame;

  const getGame = async () => {
    if (game !== null) {
      return;
    }

    await socket.awaitOpen();
    const gameRes = await socket.call('new_game');
    if (isOk(gameRes)) {
      const game = GameState.fromBinary(Uint8Array.from(gameRes.value.game));
      console.log(game);
      setGameRef.current(game);
    }
  };

  getGame();

  return game !== null ? <OnoroBoard gameState={game} /> : <>No game</>;
}
