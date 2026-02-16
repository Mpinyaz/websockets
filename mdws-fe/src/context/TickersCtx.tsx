import React, { createContext, useContext, useState, type ReactNode } from 'react'

interface Tickers {
  crypto: Array<string>
  equity: Array<string>
  forex: Array<string>
}

// Define the shape of the context (data + update functions)
interface TickerContextType {
  tickers: Tickers
  setTickers: React.Dispatch<React.SetStateAction<Tickers>>
}

const TickerContext = createContext<TickerContextType | undefined>(undefined)

export const TickerProvider = ({ children }: { children: ReactNode }) => {
  const [tickers, setTickers] = useState<Tickers>({
    crypto: [],
    equity: [],
    forex: [],
  })

  return (
    <TickerContext.Provider value={{ tickers, setTickers }}>
      {children}
    </TickerContext.Provider>
  )
}

export const useTickers = () => {
  const context = useContext(TickerContext)
  if (!context) {
    throw new Error('useTickers must be used within a TickerProvider')
  }
  return context
}
