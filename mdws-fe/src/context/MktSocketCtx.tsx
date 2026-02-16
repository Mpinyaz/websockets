import { createContext, useContext, type ReactNode } from 'react'
import { type MarketSocket, useMarketSocket } from '@/hooks/useMarketWs'

const MarketSocketContext = createContext<MarketSocket | null>(null)

export function MarketSocketProvider({ children }: { children: ReactNode }) {
  const socket = useMarketSocket()

  return (
    <MarketSocketContext.Provider value={socket}>
      {children}
    </MarketSocketContext.Provider>
  )
}

export function useMarketSocketContext() {
  const ctx = useContext(MarketSocketContext)
  if (!ctx) {
    throw new Error(
      'useMarketSocketContext must be used inside MarketSocketProvider',
    )
  }
  return ctx
}
