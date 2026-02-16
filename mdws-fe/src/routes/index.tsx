import { createFileRoute } from '@tanstack/react-router'
import { useEffect, useState } from 'react'
import type {
  CryptoUpdate,
  EquityUpdate,
  ForexUpdate,
  MarketData,
} from '@/types/updates'

import type { WsEvent } from '@/types/event'

import { useMarketSocket } from '@/hooks/useMarketWs'
import TickerTable from '@/components/TickerCard/TickerTable'

export const Route = createFileRoute('/')({
  component: Index,
})

function Index() {
  const { lastEvent } = useMarketSocket()
  const [cryptoUpdates, setCryptoUpdates] = useState<Array<CryptoUpdate>>([])
  const [forexUpdates, setForexUpdates] = useState<Array<ForexUpdate>>([])
  const [equityUpdates, setEquityUpdates] = useState<Array<EquityUpdate>>([])

  const processUpdate = <T extends { ticker: string }>(
    prev: Array<T>,
    newUpdate: T,
  ) => {
    const existingIndex = prev.findIndex(
      (update) => update.ticker === newUpdate.ticker,
    )
    if (existingIndex > -1) {
      const newUpdates = [...prev]
      newUpdates[existingIndex] = newUpdate
      return newUpdates
    } else {
      return [...prev, newUpdate]
    }
  }

  useEffect(() => {
    if (lastEvent) {
      const event = lastEvent as WsEvent<MarketData>
      if (event.type === 'market_update') {
        switch (event.assetClass) {
          case 'forex': {
            const forexUpdate: ForexUpdate = event.payload as ForexUpdate
            setForexUpdates((prev) => processUpdate(prev, forexUpdate))
            break
          }
          case 'crypto': {
            const cryptoUpdate: CryptoUpdate = event.payload as CryptoUpdate
            setCryptoUpdates((prev) => processUpdate(prev, cryptoUpdate))
            break
          }
          case 'equity': {
            const equityUpdate: EquityUpdate = event.payload as EquityUpdate
            setEquityUpdates((prev) => processUpdate(prev, equityUpdate))
            break
          }
          default:
            console.warn('Unknown assetClass:', event.assetClass, event.payload)
        }
      }
    }
  }, [lastEvent])

  return (
    <>
      <div className="container mx-auto px-4 py-8">
        <div className="text-center mb-8">
          <h1 className="font-bold text-4xl sm:text-5xl md:text-6xl lg:text-7xl xl:text-8xl text-gray-100">
            MDWS
          </h1>
          <p className="italic font-semibold text-base sm:text-lg md:text-xl text-gray-300">
            Market Data Feed using Websockets
          </p>
        </div>
        <div className="flex flex-wrap -mx-4">
          <div className="w-full sm:w-1/2 lg:w-1/3 px-4 mb-8">
            <h2 className="text-2xl font-bold text-gray-200 mb-4 text-center">
              Crypto Updates
            </h2>
            <div className="flex flex-col gap-2">
              {cryptoUpdates.map((update) => (
                <TickerTable
                  key={update.ticker}
                  data={update}
                  category="crypto"
                />
              ))}
            </div>
          </div>
          <div className="w-full sm:w-1/2 lg:w-1/3 px-4 mb-8">
            <h2 className="text-2xl font-bold text-gray-200 mb-4 text-center">
              Forex Updates
            </h2>
            <div className="flex flex-col gap-2">
              {forexUpdates.map((update) => (
                <TickerTable
                  key={update.ticker}
                  data={update}
                  category="forex"
                />
              ))}
            </div>
          </div>
          <div className="w-full sm:w-1/2 lg:w-1/3 px-4 mb-8">
            <h2 className="text-2xl font-bold text-gray-200 mb-4 text-center">
              Equity Updates
            </h2>
            <div className="flex flex-col gap-2">
              {equityUpdates.map((update) => (
                <TickerTable
                  key={update.ticker}
                  data={update}
                  category="equity"
                />
              ))}
            </div>
          </div>
        </div>
      </div>
    </>
  )
}
