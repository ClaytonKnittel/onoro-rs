import { AsyncSocketContext } from 'client/util/async_sockets';
import { Status } from 'client/util/status';
import { GameState } from 'proto/game_state';

interface ServerToClient {
  /* eslint-disable @typescript-eslint/naming-convention */
  new_game_res: (res: Status<Uint8Array>) => void;
  /* eslint-enable @typescript-eslint/naming-convention */
}

interface ClientToServer {
  /* eslint-disable @typescript-eslint/naming-convention */
  new_game_req: () => void;
  /* eslint-enable @typescript-eslint/naming-convention */
}

export type OnoroSocket = AsyncSocketContext<ServerToClient, ClientToServer>;
