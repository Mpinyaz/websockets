import { useEffect } from 'react'
import type {
  CryptoUpdate,
  EquityUpdate,
  ForexUpdate,
  MarketData,
} from '@/types/updates'
import type { TickerCategory } from '@/types/misc'
import { useTickers } from '@/context/TickersCtx'

const TickerTable = ({
  data,
  category,
}: {
  data: MarketData
  category: TickerCategory
}) => {
  const { tickers, setTickers } = useTickers()

  useEffect(() => {
    // Add ticker to the correct array if it doesn't already exist
    setTickers((prev) => {
      if (prev[category].includes(data.ticker)) {
        return prev // No change needed, skip re-render
      }
      return {
        ...prev,
        [category]: [...prev[category], data.ticker],
      }
    })
  }, [data.ticker, category, setTickers])

  // Only render table if this ticker is tracked in our state
  if (!tickers[category].includes(data.ticker)) return null

  const renderTableRows = () => {
    switch (category) {
      case 'crypto':
        const cryptoData = data as CryptoUpdate
        return (
          <>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Symbol</td>
              <td>{cryptoData.ticker}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Time</td>
              <td>{cryptoData.date}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Exchange</td>
              <td>{cryptoData.exchange}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Last Size</td>
              <td>{cryptoData.lastSize}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Last Price</td>
              <td>${cryptoData.lastPrice.toLocaleString()}</td>
            </tr>
          </>
        )
      case 'forex':
        const forexData = data as ForexUpdate
        return (
          <>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Symbol</td>
              <td>{forexData.ticker}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Time</td>
              <td>{forexData.timestamp}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Bid Size</td>
              <td>{forexData.bidSize}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Bid Price</td>
              <td>${forexData.bidPrice.toLocaleString()}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Mid Price</td>
              <td>${forexData.midPrice.toLocaleString()}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Ask Price</td>
              <td>${forexData.askPrice.toLocaleString()}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Ask Size</td>
              <td>{forexData.askSize}</td>
            </tr>
          </>
        )
      case 'equity':
        const equityData = data as EquityUpdate
        return (
          <>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Symbol</td>
              <td>{equityData.ticker}</td>
            </tr>
            <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
              <td className="font-bold">Time</td>
              <td>{equityData.date}</td>
            </tr>
            {equityData.lastPrice && (
              <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
                <td className="font-bold">Last Price</td>
                <td>${equityData.lastPrice.toLocaleString()}</td>
              </tr>
            )}
            {equityData.lastSize && (
              <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
                <td className="font-bold">Last Size</td>
                <td>{equityData.lastSize}</td>
              </tr>
            )}
            {equityData.bidPrice && (
              <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
                <td className="font-bold">Bid Price</td>
                <td>${equityData.bidPrice.toLocaleString()}</td>
              </tr>
            )}
            {equityData.bidSize && (
              <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
                <td className="font-bold">Bid Size</td>
                <td>{equityData.bidSize}</td>
              </tr>
            )}
            {equityData.askPrice && (
              <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
                <td className="font-bold">Ask Price</td>
                <td>${equityData.askPrice.toLocaleString()}</td>
              </tr>
            )}
            {equityData.askSize && (
              <tr className="px-4 py-2 even:bg-gray-800 odd:bg-gray-900">
                <td className="font-bold">Ask Size</td>
                <td>{equityData.askSize}</td>
              </tr>
            )}
          </>
        )
      default:
        return null
    }
  }

  return (
    <div className="rounded-xl border border-gray-700 shadow-lg px-4 py-3 bg-gradient-to-br from-gray-800 to-gray-900">
      <table className="w-full text-left text-white">
        <thead>
          <tr className="bg-gray-700">
            <th
              scope="col"
              className="px-4 py-2 border-b border-gray-600 rounded-tl-lg"
            >
              Field
            </th>
            <th
              scope="col"
              className="px-4 py-2 border-b border-gray-600 rounded-tr-lg"
            >
              Value
            </th>
          </tr>
        </thead>
        <tbody>{renderTableRows()}</tbody>
        <caption className="text-sm font-semibold text-gray-300 mb-2">
          Data provided by Tiingo
        </caption>
      </table>
    </div>
  )
}

export default TickerTable
