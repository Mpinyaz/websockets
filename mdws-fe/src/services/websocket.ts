import { io } from 'socket.io-client'

const WS_URL = import.meta.env.VITE_WS_URL ?? 'http://localhost:8080/ws'

export const socket = io(WS_URL)
