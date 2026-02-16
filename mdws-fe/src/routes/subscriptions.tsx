import { createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'
import { useTickers } from '@/context'
import { useMarketSocketContext } from '@/context/MktSocketCtx'

export const Route = createFileRoute('/subscriptions')({
  component: Subscriptions,
})

const predefinedTickers = {
  crypto: ['btcusd', 'ethusd', 'solusd', 'dogeusd'],
  forex: ['usdjpy', 'usdzar', 'eurzar'],
  equity: ['aapl', 'msft', 'googl'],
}

function Subscriptions() {
  const { tickers, setTickers } = useTickers()
  const { subscribe, unsubscribe } = useMarketSocketContext()
  const [selectedAssetClass, setSelectedAssetClass] =
    useState<keyof typeof predefinedTickers>('crypto')
  const [selectedTicker, setSelectedTicker] = useState('')

  const handleAddToTickers = () => {
    if (
      selectedTicker &&
      !tickers[selectedAssetClass].includes(selectedTicker)
    ) {
      setTickers((prevTickers) => ({
        ...prevTickers,
        [selectedAssetClass]: [
          ...prevTickers[selectedAssetClass],
          selectedTicker,
        ],
      }))
      subscribe({ assetClass: selectedAssetClass, tickers: [selectedTicker] })
      setSelectedTicker('') // Clear selected ticker after adding
    }
  }

  const handleRemoveTicker = (
    assetClass: keyof typeof predefinedTickers,
    tickerToRemove: string,
  ) => {
    setTickers((prevTickers) => {
      const updatedTickers = {
        ...prevTickers,
        [assetClass]: prevTickers[assetClass].filter(
          (ticker) => ticker !== tickerToRemove,
        ),
      }
      return updatedTickers
    })
    unsubscribe({ assetClass, tickers: [tickerToRemove] })
  }

  return (
    <>
      <section className="flex flex-col justify-center items-center p-4">
        <h1 className="text-3xl font-bold mb-6">Manage Ticker Symbols</h1>
        <div className="flex flex-col sm:flex-row space-y-4 sm:space-y-0 sm:space-x-4 mb-8">
          <select
            value={selectedAssetClass}
            onChange={(e) => {
              setSelectedAssetClass(
                e.target.value as keyof typeof predefinedTickers,
              )
              setSelectedTicker('') // Reset selected ticker when asset class changes
            }}
            className="p-2 border rounded-md bg-gray-700 text-gray-100 border-gray-600 focus:ring-blue-500 focus:border-blue-500"
          >
            {Object.keys(predefinedTickers).map((assetClass) => (
              <option key={assetClass} value={assetClass}>
                {assetClass.charAt(0).toUpperCase() + assetClass.slice(1)}
              </option>
            ))}
          </select>

          <select
            value={selectedTicker}
            onChange={(e) => setSelectedTicker(e.target.value)}
            className="p-2 border rounded-md bg-gray-700 text-gray-100 border-gray-600 focus:ring-blue-500 focus:border-blue-500"
            disabled={!predefinedTickers[selectedAssetClass].length}
          >
            <option value="">Select a ticker</option>
            {predefinedTickers[selectedAssetClass].map((ticker) => (
              <option key={ticker} value={ticker}>
                {ticker.toUpperCase()}
              </option>
            ))}
          </select>

          <button
            onClick={handleAddToTickers}
            className="px-5 py-2 bg-blue-600 text-white font-semibold rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-gray-800 transition ease-in-out duration-150"
          >
            Add Ticker
          </button>
        </div>

        <div className="w-full max-w-2xl">
          {Object.keys(tickers).map((assetClassKey) => {
            const assetClass = assetClassKey as keyof typeof tickers
            if (tickers[assetClass].length === 0) {
              return null
            }
            return (
              <div key={assetClass} className="mb-4">
                <h2 className="text-xl font-semibold mb-2 text-gray-200">
                  {assetClass.charAt(0).toUpperCase() + assetClass.slice(1)}
                </h2>
                <div className="flex flex-wrap gap-2">
                  {tickers[assetClass].map((ticker) => (
                    <span
                      key={ticker}
                      className="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-blue-800 text-blue-100 max-w-full overflow-hidden text-ellipsis whitespace-nowrap"
                    >
                      {ticker.toUpperCase()}
                      <button
                        onClick={() => handleRemoveTicker(assetClass, ticker)}
                        className="ml-2 -mr-0.5 inline-flex items-center justify-center h-4 w-4 rounded-full text-blue-100 hover:bg-blue-700 hover:text-white focus:outline-none focus:bg-blue-700 transition ease-in-out duration-150"
                      >
                        <svg
                          className="h-2 w-2"
                          stroke="currentColor"
                          fill="none"
                          viewBox="0 0 8 8"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth="1.5"
                            d="M1 1l6 6m0-6L1 7"
                          />
                        </svg>
                      </button>
                    </span>
                  ))}
                </div>
              </div>
            )
          })}
        </div>
      </section>
    </>
  )
}
