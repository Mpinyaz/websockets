import type { WsEvent } from './event.ts'

export interface ForexUpdate {
  type: 'Q' // Quote
  ticker: string
  timestamp: string // ISO
  bidSize: number
  bidPrice: number
  midPrice: number
  askPrice: number
  askSize: number
}

export interface CryptoUpdate {
  update_type: 'T' | 'Q' // Trade or Quote
  ticker: string
  date: string // ISO
  exchange: string
  last_size: number
  last_price: number
}

export interface EquityUpdate {
  updateType: 'T' | 'Q' | 'B'
  date: string // ISO
  nanoseconds: number
  ticker: string

  bidSize?: number
  bidPrice?: number
  midPrice?: number
  askPrice?: number
  askSize?: number

  lastPrice?: number
  lastSize?: number

  halted: number
  afterHours: number
  iso: number
  oddlot?: number
  nmsRule611?: number
}

export type MarketData = ForexUpdate | CryptoUpdate | EquityUpdate

export type MarketUpdateEvent = WsEvent<MarketData>
