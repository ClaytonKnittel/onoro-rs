export enum StatusCode {
  Ok = 'Ok',

  MessageTimeout = 'MessageTimeout',
  NotFound = 'NotFound',
}

export interface OkStatusT<T = null> {
  readonly status: StatusCode.Ok;
  readonly value: T;
}

export type ErrStatusCode = Exclude<StatusCode, StatusCode.Ok>;

export interface ErrStatusT {
  readonly status: ErrStatusCode;
  readonly message: string;
}

export type Status<T = null> = OkStatusT<T> | ErrStatusT;

export interface SerializedStatus {
  readonly status: StatusCode;
  readonly payload: unknown;
}

export function makeErrStatus(
  status: ErrStatusCode,
  message: string
): ErrStatusT {
  return { status, message };
}

export function statusFromError<E extends Error, T>(
  error: E | null,
  code: ErrStatusCode,
  value_on_success: T
): Status<T> {
  return error === null
    ? makeOkStatus<T>(value_on_success)
    : makeErrStatus(code, error.message);
}

export const OkStatus: OkStatusT = {
  status: StatusCode.Ok,
  value: null,
};

export function makeOkStatus<T>(value: T): OkStatusT<T> {
  return { status: StatusCode.Ok, value };
}

export function isOk<T>(status: Status<T>): status is OkStatusT<T> {
  return status.status === StatusCode.Ok;
}

export function isStatus(status: unknown): status is Status<unknown> {
  return (
    status !== null &&
    typeof status === 'object' &&
    'status' in status &&
    typeof status.status === 'string' &&
    status.status in StatusCode &&
    (((status.status as StatusCode) === StatusCode.Ok && 'value' in status) ||
      ('message' in status && typeof status.message === 'string'))
  );
}

export function isSerializedStatus(
  status: unknown
): status is SerializedStatus {
  return (
    status !== null &&
    typeof status === 'object' &&
    'status' in status &&
    'payload' in status &&
    typeof status.status === 'string' &&
    status.status in StatusCode &&
    ((status.status as StatusCode) === StatusCode.Ok ||
      typeof status.payload === 'string')
  );
}

export function serializeStatus(status: Status<unknown>): SerializedStatus {
  if (isOk(status)) {
    return { status: status.status, payload: status.value };
  } else {
    return { status: status.status, payload: status.message };
  }
}

export function deserializeStatus(
  status: SerializedStatus
): Status<unknown> | null {
  if (status.status === StatusCode.Ok) {
    return { status: status.status, value: status.payload };
  } else {
    return { status: status.status, message: status.payload as string };
  }
}
