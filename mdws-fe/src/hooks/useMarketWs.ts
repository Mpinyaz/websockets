// hooks/useMarketSocket.ts
import useWebSocket, { ReadyState } from 'react-use-websocket'
import type { WsEvent } from '../types/event'
import type { SubscribePayload } from '../types/subscriptions'
import type { WebSocketLike } from 'react-use-websocket/dist/lib/types'

const WS_URL = import.meta.env.VITE_WS_URL ?? 'ws://localhost:8080/ws'

export interface MarketSocket {
  connected: boolean
  lastEvent: WsEvent | null
  subscribe: (payload: SubscribePayload) => void
  unsubscribe: (assetClass: SubscribePayload['assetClass']) => void
  getWebSocket: () => WebSocketLike | null
}

export function useMarketSocket(): MarketSocket {
  const { sendJsonMessage, lastJsonMessage, readyState, getWebSocket } =
    useWebSocket<WsEvent>(WS_URL, {
      shouldReconnect: () => true,
      reconnectInterval: 2000,
      reconnectAttempts: 10,
      heartbeat: {
        message: JSON.stringify({ type: 'ping' }),
        interval: 30_000,
        returnMessage: 'pong',
      },
      // Add these options to prevent duplicate connections
      share: true, // Share WebSocket instance across multiple hooks
      filter: () => true,
    })

  const subscribe = (payload: SubscribePayload) => {
    if (readyState === ReadyState.OPEN) {
      sendJsonMessage({
        type: 'subscribe',
        payload,
      })
    }
  }

  const unsubscribe = (assetClass: SubscribePayload['assetClass']) => {
    if (readyState === ReadyState.OPEN) {
      sendJsonMessage({
        type: 'unsubscribe',
        payload: { assetClass, tickers: [] },
      })
    }
  }

  return {
    connected: readyState === ReadyState.OPEN,
    lastEvent: lastJsonMessage,
    subscribe,
    unsubscribe,
    getWebSocket,
  }
}
