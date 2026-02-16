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
      case 'crypto': {
        const cryptoData = data as CryptoUpdate
        return (
          <>
            <tr className="border-b border-gray-700">
              <td className="font-bold p-2">Time</td>
              <td className="p-2 text-right">{cryptoData.date}</td>
            </tr>
            <tr className="border-b border-gray-700">
              <td className="font-bold p-2">Exchange</td>
              <td className="p-2 text-right">{cryptoData.exchange}</td>
            </tr>
            {cryptoData.last_size && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Last Size</td>
                <td className="p-2 text-right">{cryptoData.last_size}</td>
              </tr>
            )}
            {cryptoData.last_price && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Last Price</td>
                <td className="p-2 text-right">
                  ${cryptoData.last_price.toLocaleString()}
                </td>
              </tr>
            )}
          </>
        )
      }
      case 'forex': {
        const forexData = data as ForexUpdate
        return (
          <>
            <tr className="border-b border-gray-700">
              <td className="font-bold p-2">Time</td>
              <td className="p-2 text-right">{forexData.timestamp}</td>
            </tr>
            {forexData.bidSize && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Bid Size</td>
                <td className="p-2 text-right">{forexData.bidSize}</td>
              </tr>
            )}
            {forexData.bidPrice && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Bid Price</td>
                <td className="p-2 text-right">
                  ${forexData.bidPrice.toLocaleString()}
                </td>
              </tr>
            )}
            {forexData.midPrice && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Mid Price</td>
                <td className="p-2 text-right">
                  ${forexData.midPrice.toLocaleString()}
                </td>
              </tr>
            )}
            {forexData.askPrice && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Ask Price</td>
                <td className="p-2 text-right">
                  ${forexData.askPrice.toLocaleString()}
                </td>
              </tr>
            )}
            {forexData.askSize && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Ask Size</td>
                <td className="p-2 text-right">{forexData.askSize}</td>
              </tr>
            )}
          </>
        )
      }
      case 'equity': {
        const equityData = data as EquityUpdate
        return (
          <>
            <tr className="border-b border-gray-700">
              <td className="font-bold p-2">Time</td>
              <td className="p-2 text-right">{equityData.date}</td>
            </tr>
            {equityData.lastPrice && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Last Price</td>
                <td className="p-2 text-right">
                  ${equityData.lastPrice.toLocaleString()}
                </td>
              </tr>
            )}
            {equityData.lastSize && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Last Size</td>
                <td className="p-2 text-right">{equityData.lastSize}</td>
              </tr>
            )}
            {equityData.bidPrice && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Bid Price</td>
                <td className="p-2 text-right">
                  ${equityData.bidPrice.toLocaleString()}
                </td>
              </tr>
            )}
            {equityData.bidSize && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Bid Size</td>
                <td className="p-2 text-right">{equityData.bidSize}</td>
              </tr>
            )}
            {equityData.askPrice && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Ask Price</td>
                <td className="p-2 text-right">
                  ${equityData.askPrice.toLocaleString()}
                </td>
              </tr>
            )}
            {equityData.askSize && (
              <tr className="border-b border-gray-700">
                <td className="font-bold p-2">Ask Size</td>
                <td className="p-2 text-right">{equityData.askSize}</td>
              </tr>
            )}
          </>
        )
      }
      default:
        return null
    }
  }

  return (
    <div className="rounded-xl border border-gray-700 shadow-lg p-6 bg-gray-800">
      <h2 className="text-2xl font-bold mb-4 text-gray-100">
        {data.ticker.toUpperCase()}
      </h2>
                <div className="rounded-lg">
                  <table className="w-full text-left text-white">
                    <thead>
                      <tr className="bg-gray-700">
                        <th
                          scope="col"
                          className="p-2 border-b border-gray-600 rounded-tl-lg"
                        >
                          Field
                        </th>
                        <th
                          scope="col"
                          className="p-2 border-b border-gray-600 rounded-tr-lg"
                        >
                          Value
                        </th>
                      </tr>
                    </thead>          <tbody>{renderTableRows()}</tbody>
        </table>
      </div>
    </div>
  )
}

export default TickerTable
