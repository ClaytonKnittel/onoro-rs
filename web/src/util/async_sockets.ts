/* eslint-disable @typescript-eslint/no-explicit-any */
import { DeepReadonly } from 'ts-essentials';
import { v4 as uuidv4 } from 'uuid';

import {
  Status,
  StatusCode,
  deserializeStatus,
  isSerializedStatus,
  isStatus,
  makeErrStatus,
  serializeStatus,
} from 'client/util/status';

interface EventsMap {
  [event_name: string]: any;
}

interface EmitMessage<Params extends unknown[]> {
  event: string;
  args: Params | null;
}

interface CallMessage<Params extends unknown[]> {
  event: string;
  uuid: string;
  args: Params | null;
}

interface ResponseMessage<T> {
  uuid: string;
  status: Status<T>;
}

interface SocketMessage {
  emit?: EmitMessage<unknown[]>;
  call?: CallMessage<unknown[]>;
  response?: ResponseMessage<unknown>;
}

function isEmitMessage<Params extends unknown[]>(
  message: unknown
): message is EmitMessage<Params> {
  return (
    message !== null &&
    typeof message === 'object' &&
    'event' in message &&
    typeof message.event === 'string' &&
    'args' in message &&
    Array.isArray(message.args)
  );
}

function isCallMessage<Params extends unknown[]>(
  message: unknown
): message is CallMessage<Params> {
  return (
    message !== null &&
    typeof message === 'object' &&
    'event' in message &&
    typeof message.event === 'string' &&
    'uuid' in message &&
    typeof message.uuid === 'string' &&
    'args' in message &&
    Array.isArray(message.args)
  );
}

function isResponseMessage<Params extends unknown[]>(
  message: unknown
): message is ResponseMessage<Params> {
  return (
    message !== null &&
    typeof message === 'object' &&
    'uuid' in message &&
    typeof message.uuid === 'string' &&
    'status' in message &&
    isStatus(message.status)
  );
}

function isMessage(message: unknown): message is SocketMessage {
  return (
    message !== null &&
    typeof message === 'object' &&
    (('emit' in message && isEmitMessage(message.emit)) ||
      ('call' in message && isCallMessage(message.call)) ||
      ('response' in message && isResponseMessage(message.response)))
  );
}

type EventNames<Map extends EventsMap> = keyof Map & string;

type ToReqEventName<EmitEventName extends string> = `${EmitEventName}_req`;
type ToResEventName<EmitEventName extends string> = `${EmitEventName}_res`;

type InternalCallback<Params extends Parameters<any>> = (
  uuid: string,
  ...args: Params
) => void;

type ToRequestEvents<Events extends EventsMap> = {
  [Ev in keyof Events as Ev extends `${infer T}_req`
    ? T & string
    : never]: InternalCallback<Parameters<Events[Ev]>>;
};

// Response events must be of the form '<event>_res': (status: Status<T>) => void
type ToResponseEvents<Events extends EventsMap> = {
  [Ev in keyof Events as Ev extends `${infer T}_res`
    ? T & string
    : never]: Events[Ev] extends (result: Status<infer T>) => void
    ? [result: Status<T>]
    : never;
};

type ReqParams<
  EventName extends keyof Events & string,
  Events extends EventsMap,
> = Parameters<Events[ToReqEventName<EventName>]>;

type ResponseStatus<
  EventName extends keyof Events & string,
  Events extends EventsMap,
> = Events[ToResEventName<EventName>] extends (result: Status<infer T>) => void
  ? Status<T>
  : never;

interface EmitListenerInfo<
  Events extends EventsMap,
  EventName extends keyof Events & string,
> {
  type: 'emit';
  callback: Events[EventName];
}

interface CallListenerInfo<
  ListenEvents extends EventsMap,
  EmitEvents extends EventsMap,
  EventName extends CallResponseEvents<EmitEvents, ListenEvents>,
> {
  type: 'call';
  callback: (
    ...args: ReqParams<EventName, ListenEvents>
  ) => Promise<DeepReadonly<ResponseStatus<EventName, EmitEvents>>>;
}

/**
 * Listener info is the type of object held in the listeners dictionary. These
 * are always active and don't need to be prompted by sending another message
 * before (unlike response, which needs to be prompted by a call).
 */
type ListenerInfo<
  ListenEvents extends EventsMap,
  EmitEvents extends EventsMap,
  EventName extends string,
> =
  | EmitListenerInfo<ListenEvents, EventName & keyof ListenEvents>
  | CallListenerInfo<
      ListenEvents,
      EmitEvents,
      EventName & CallResponseEvents<EmitEvents, ListenEvents>
    >;

// CallResponseEvents are events that have a *_req form in ReqEvents and a
// *_res form in ResEvents.
type CallResponseEvents<
  ResEvents extends EventsMap,
  ReqEvents extends EventsMap,
> = EventNames<ToRequestEvents<ReqEvents>> &
  EventNames<ToResponseEvents<ResEvents>>;

type EmitEventNames<
  ListenEvents extends EventsMap,
  EmitEvents extends EventsMap,
> = Exclude<
  EventNames<EmitEvents>,
  | ToReqEventName<CallResponseEvents<ListenEvents, EmitEvents>>
  | ToResEventName<CallResponseEvents<EmitEvents, ListenEvents>>
>;

type OnEventNames<
  ListenEvents extends EventsMap,
  EmitEvents extends EventsMap,
> = Exclude<
  EventNames<ListenEvents>,
  | ToResEventName<CallResponseEvents<ListenEvents, EmitEvents>>
  | ToReqEventName<CallResponseEvents<EmitEvents, ListenEvents>>
>;

interface ResponseMessageInfo<
  ListenEvents extends EventsMap,
  EmitEvents extends EventsMap,
  EventName extends CallResponseEvents<ListenEvents, EmitEvents>,
> {
  timeoutId: NodeJS.Timeout;
  resolve: (status: ResponseStatus<EventName, ListenEvents>) => void;
}

export class AsyncSocketContext<
  ListenEvents extends EventsMap,
  EmitEvents extends EventsMap,
> {
  private readonly url: string;
  private socket: WebSocket;
  private timeout: number;
  private verbose: boolean;
  private isOpen: boolean;

  /**
   * A map of event names to the corresponding call/response or emit listeners.
   */
  private readonly listeners: Map<
    string,
    ListenerInfo<ListenEvents, EmitEvents, never>
  >;

  /**
   * A map of uuids to messages that were called with `call`.
   */
  private readonly outstanding_calls: Map<
    string,
    ResponseMessageInfo<ListenEvents, EmitEvents, never>
  >;

  constructor(url: string, verbose?: boolean) {
    this.url = url;
    this.socket = new WebSocket(url, ['websocket', 'polling']);
    this.isOpen = false;
    this.timeout = 1000;
    this.verbose = verbose ?? false;
    this.initializeWebSocket();

    this.listeners = new Map();
    this.outstanding_calls = new Map();
  }

  async awaitOpen(): Promise<void> {
    return new Promise((resolve) => {
      if (this.isOpen) {
        resolve();
      } else {
        this.socket.addEventListener('open', () => {
          resolve();
        });
      }
    });
  }

  private onOpen() {
    if (this.verbose) {
      console.log('websocket opened');
    }
    this.isOpen = true;
  }

  private handleEmit(message: EmitMessage<unknown[]>) {
    const eventInfo = this.listeners.get(message.event);
    if (eventInfo === undefined) {
      console.error('Unknown emit event:', message.event);
      return;
    }
    if (eventInfo.type !== 'emit') {
      console.error('Received emit for call/response message:', message.event);
      return;
    }

    const callback: (...args: any[]) => void = eventInfo.callback;
    callback(...(message.args ?? []));
  }

  private async handleCall(message: CallMessage<unknown[]>) {
    const eventInfo = this.listeners.get(message.event);
    if (eventInfo === undefined) {
      console.error('Unknown call event:', message.event);
      return;
    }
    if (eventInfo.type !== 'call') {
      console.error('Received call for emit message:', message.event);
      return;
    }

    const callback: (...args: any) => Promise<Status<unknown>> =
      eventInfo.callback;
    const status = await callback(...(message.args ?? []));

    if (this.verbose) {
      console.log(`responding to ${message.event} with`, status);
    }
    const response: ResponseMessage<unknown> = {
      uuid: message.uuid,
      status,
    };
    this.sendMessage({ response });
  }

  private async handleResponse({ uuid, status }: ResponseMessage<unknown>) {
    if (this.verbose) {
      console.log(uuid, status);
    }
    if (!this.outstanding_calls.has(uuid)) {
      console.error(`Error: received event with unknown uuid: ${uuid}`);
      return;
    }

    const messageInfo = this.outstanding_calls.get(uuid) as ResponseMessageInfo<
      ListenEvents,
      EmitEvents,
      CallResponseEvents<ListenEvents, EmitEvents>
    >;
    clearTimeout(messageInfo.timeoutId);
    messageInfo.resolve(
      status as ResponseStatus<
        CallResponseEvents<ListenEvents, EmitEvents>,
        ListenEvents
      >
    );
  }

  private async onMessage(event: MessageEvent) {
    const message = this.parseMessage(event.data);
    if (!isMessage(message)) {
      console.log('ill-formed message:', message);
      return;
    }

    if (message.emit !== undefined) {
      this.handleEmit(message.emit);
    } else if (message.call !== undefined) {
      this.handleCall(message.call);
    } else if (message.response !== undefined) {
      this.handleResponse(message.response);
    }
  }

  private onError() {
    console.error('websocket closed via error');
    this.isOpen = false;
  }

  private onClose() {
    console.log('websocket closed');
    this.isOpen = false;
    this.periodicallyTryReconnect();
  }

  private initializeWebSocket() {
    this.socket.onopen = () => {
      this.onOpen();
    };
    this.socket.onmessage = (event) => {
      this.onMessage(event);
      return false;
    };
    this.socket.onerror = () => {
      this.onError();
    };
    this.socket.onclose = () => {
      this.onClose();
    };
  }

  private periodicallyTryReconnect() {
    // TODO: implement reconnect logic
    // this.socket = new WebSocket(this.url)
    // this.initializeWebSocket()
  }

  private sendMessage(data: SocketMessage) {
    this.socket.send(
      JSON.stringify(data, (_key, value) => {
        if (isStatus(value)) {
          return serializeStatus(value);
        }

        return value;
      })
    );
  }

  private parseMessage(data: unknown): unknown {
    if (typeof data !== 'string') {
      return null;
    }
    try {
      return JSON.parse(data, (_key, value) => {
        if (isSerializedStatus(value)) {
          return deserializeStatus(value);
        }

        return value;
      });
    } catch {
      return null;
    }
  }

  private addTimeout<
    EventName extends CallResponseEvents<ListenEvents, EmitEvents>,
  >(
    eventName: string,
    uuid: string,
    timeoutMs: number,
    callback: (response: ResponseStatus<EventName, ListenEvents>) => void
  ): NodeJS.Timeout {
    return setTimeout(() => {
      this.outstanding_calls.delete(uuid);

      callback(
        makeErrStatus(
          StatusCode.MessageTimeout,
          `Async socket call ${eventName} timed out after ${
            timeoutMs / 1000
          } second${timeoutMs === 1000 ? '' : 's'}`
        ) as ResponseStatus<EventName, ListenEvents>
      );
    }, timeoutMs);
  }

  emit<EventName extends EmitEventNames<ListenEvents, EmitEvents>>(
    eventName: EventName,
    ...args: Parameters<EmitEvents[EventName]>
  ) {
    const emit: EmitMessage<Parameters<EmitEvents[EventName]>> = {
      event: eventName,
      args: args.length === 0 ? null : args,
    };
    this.sendMessage({ emit });
  }

  on<EventName extends OnEventNames<ListenEvents, EmitEvents>>(
    eventName: EventName,
    callback: ListenEvents[EventName]
  ) {
    const alias = this.listeners as Map<
      EventName,
      ListenerInfo<ListenEvents, EmitEvents, EventName>
    >;
    alias.set(eventName, {
      type: 'emit',
      callback,
    });
  }

  async call<EventName extends CallResponseEvents<ListenEvents, EmitEvents>>(
    eventName: EventName,
    ...args: ReqParams<EventName, EmitEvents>
  ): Promise<ResponseStatus<EventName, ListenEvents>> {
    if (this.verbose) {
      console.log(`calling ${eventName} with`, args);
    }

    return new Promise((resolve) => {
      const uuid = uuidv4();
      const timeoutId = this.addTimeout(eventName, uuid, this.timeout, resolve);

      this.outstanding_calls.set(uuid, { timeoutId, resolve });

      const call: CallMessage<ReqParams<EventName, EmitEvents>> = {
        event: eventName,
        uuid,
        args: args.length === 0 ? null : args,
      };
      this.sendMessage({ call });
    });
  }

  respond<EventName extends CallResponseEvents<EmitEvents, ListenEvents>>(
    eventName: EventName,
    callback: (
      ...args: ReqParams<EventName, ListenEvents>
    ) => Promise<DeepReadonly<ResponseStatus<EventName, EmitEvents>>>
  ): void {
    const alias = this.listeners as Map<
      EventName,
      CallListenerInfo<ListenEvents, EmitEvents, EventName>
    >;
    alias.set(eventName, {
      type: 'call',
      callback,
    });
  }
}
