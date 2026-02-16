export interface WsEvent<T = unknown> {
  type: string
  payload: T
  from?: string
  time?: string
  assetClass?: string
}
