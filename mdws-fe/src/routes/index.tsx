import { createFileRoute } from '@tanstack/react-router'
import { useEffect, useState } from 'react'
import type { MarketData, CryptoUpdate, EquityUpdate, ForexUpdate } from '@/types/updates'
import type { TickerCategory } from '@/types/misc'
import type { WsEvent } from '@/types/event'

import { useMarketSocket } from '@/hooks/useMarketWs'
import TickerTable from '@/components/TickerCard/TickerTable'
export const Route = createFileRoute('/')({
  component: Index,
})

function Index() {
  const { lastEvent } = useMarketSocket()
  const [cryptoUpdates, setCryptoUpdates] = useState<CryptoUpdate[]>([])

  // const subscribeMemo = useCallback(subscribe, [subscribe])
  // const unsubscribeMemo = useCallback(unsubscribe, [unsubscribe])
  //
  // useEffect(() => {
  //   if (connected) {
  //     subscribeMemo({ assetClass: 'crypto', tickers: [] })
  //   }
  //   return () => {
  //     if (connected) {
  //       unsubscribeMemo('crypto')
  //     }
  //   }
  // }, [connected, subscribeMemo, unsubscribeMemo])

  useEffect(() => {
    if (lastEvent) {
      const event = lastEvent as WsEvent
      if (event.type === 'market_update' && event.payload) {
        const payload = event.payload as any // TODO: Refine this type
        const marketUpdate: CryptoUpdate = { // TODO: Handle other types
          updateType: payload.update_type,
          ticker: payload.ticker,
          date: payload.date,
          exchange: payload.exchange,
          lastSize: payload.last_size,
          lastPrice: payload.last_price,
        }
        setCryptoUpdates((prev) => {
          const existingIndex = prev.findIndex(
            (update) => update.ticker === marketUpdate.ticker,
          )
          if (existingIndex > -1) {
            const newUpdates = [...prev]
            newUpdates[existingIndex] = marketUpdate
            return newUpdates
          } else {
            return [...prev, marketUpdate]
          }
        })
      }
    }
  }, [lastEvent])

  return (
    <>
      <div className="container flex flex-col">
        <div className="text-center">
          <h1 className="font-bold text-8xl">MDWS</h1>
          <p className="italic font-semibold">
            Market Data Feed using Websockets
          </p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 p-4">
          {cryptoUpdates.map((update) => (
            <TickerTable key={update.ticker} data={update} category="crypto" />
          ))}
        </div>
      </div>
    </>
  )
}
