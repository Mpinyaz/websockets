import { StrictMode } from 'react'
import ReactDOM from 'react-dom/client'
import { RouterProvider, createRouter } from '@tanstack/react-router'

// Import the generated route tree
import { routeTree } from './routeTree.gen'

import './styles.css'
import reportWebVitals from './reportWebVitals.ts'
import { MarketSocketProvider } from './context/MktSocketCtx.tsx'
import { TickerProvider } from './context/TickersCtx.tsx'

import { useMarketSocket } from '@hooks/useMarketWs'
import type { MarketSocket } from '@hooks/useMarketWs'
import { Link } from 'lucide-react'

export interface RouterContext {
  socket: MarketSocket
}

// Create a new router instance
const router = createRouter({
  routeTree,
  context: {
    socket: undefined!,
  },
  defaultPreload: 'intent',
  scrollRestoration: true,
  defaultStructuralSharing: true,
  defaultPreloadStaleTime: 0,
  defaultNotFoundComponent: () => {
    return (
      <div>
        <p>Not found!</p>
        <Link to="/">Go home</Link>
      </div>
    )
  },
})

// Register the router instance for type safety
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

function App() {
  const marketSocket = useMarketSocket()

  return <RouterProvider router={router} context={{ socket: marketSocket }} />
}

const rootElement = document.getElementById('app')

if (rootElement && !rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement)

  root.render(
    <StrictMode>
      <TickerProvider>
        <MarketSocketProvider>
          <App />
        </MarketSocketProvider>
      </TickerProvider>
    </StrictMode>,
  )
}

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
