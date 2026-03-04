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
  updateType: 'T' | 'Q' | 'B' // "T" = trade, "Q" = quote, "B" = bar
  date: string // RFC3339 timestamp
  nanoseconds: number
  ticker: string

  // Quote fields
  bidSize?: number
  bidPrice?: number
  midPrice?: number
  askPrice?: number
  askSize?: number
  bidExchange?: string
  askExchange?: string

  // Trade fields
  lastPrice?: number
  lastSize?: number
  tradeId?: number
  exchange?: string

  // Bar fields
  open?: number
  high?: number
  low?: number
  close?: number
  volume?: number
  tradeCount?: number
  vwap?: number

  // Common
  conditions?: string[]
  tape?: string
}

export type MarketData = ForexUpdate | CryptoUpdate | EquityUpdate

export type MarketUpdateEvent = WsEvent<MarketData>
