import type { WsEvent } from './event'

export type AssetClass = 'forex' | 'equity' | 'crypto' | 'crypto_data'

export interface SubscribePayload {
  assetClass: AssetClass
  tickers: Array<string>
}

export type UnsubscribePayload = SubscribePayload

export type UnsubscribeEvent = WsEvent<UnsubscribePayload>

export interface SubscribeResponse {
  asset: AssetClass
  status: string
  symbol: Array<string>
}
