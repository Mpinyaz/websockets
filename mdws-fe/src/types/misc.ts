import type { AssetClass } from './subscriptions'
import type { WsEvent } from './event'

export type TickerCategory = 'crypto' | 'forex' | 'equity'

export interface AckPayload {
  status: 'subscribed' | 'unsubscribed' | 'patched'
  asset?: AssetClass
  symbols?: Array<string>
}

export type AckEvent = WsEvent<AckPayload>

export interface ErrorPayload {
  message: string
}

export type ErrorEvent = WsEvent<ErrorPayload>
