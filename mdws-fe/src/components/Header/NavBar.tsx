import { useState } from 'react'
import { Circle, Home, Menu, X } from 'lucide-react'
import { NavLink } from './NavLink'
import { useMarketSocketContext } from '@/context/MktSocketCtx'

export default function Navbar() {
  const socket = useMarketSocketContext()
  const [isOpen, setIsOpen] = useState(false)
  const isConnected = socket.connected

  return (
    <>
      <nav className="bg-linear-to-r from-slate-900 to-slate-800 border-b border-slate-700 p-2">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <NavLink
            to="/"
            className="font-bold text-3xl text-transparent bg-clip-text bg-linear-to-r from-cyan-400 to-blue-500 hover:from-cyan-300 hover:to-blue-400 transition-all"
          >
            MDWS
          </NavLink>

          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2 px-4 py-2 rounded-lg bg-slate-800/80 border border-slate-600/50 backdrop-blur-sm">
              <div className="relative">
                <Circle
                  size={12}
                  className={`${
                    isConnected
                      ? 'fill-green-500 text-green-500'
                      : 'fill-red-500 text-red-500'
                  }`}
                />
                {isConnected && (
                  <Circle
                    size={12}
                    className="absolute inset-0 fill-green-500 text-green-500 animate-ping opacity-75"
                  />
                )}
              </div>
              <span
                className={`text-xs font-semibold tracking-wider ${
                  isConnected ? 'text-green-400' : 'text-red-400'
                }`}
              >
                {isConnected ? 'LIVE' : 'OFFLINE'}
              </span>
            </div>

            <button
              onClick={() => setIsOpen(true)}
              className="p-2.5 rounded-lg bg-slate-800 hover:bg-slate-700 border border-slate-600 transition-all hover:scale-105 active:scale-95"
              aria-label="Open menu"
            >
              <Menu size={22} className="text-slate-200" />
            </button>
          </div>
        </div>
      </nav>

      <div
        className={`fixed inset-0 bg-black/60 backdrop-blur-sm transition-opacity duration-300 z-40 ${
          isOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'
        }`}
        onClick={() => setIsOpen(false)}
      />

      <aside
        className={`fixed top-0 right-0 h-full w-80 bg-linear-to-b from-slate-900 to-slate-950 border-l border-slate-700 shadow-2xl z-50 flex flex-col transition-transform duration-300 ease-out ${
          isOpen ? 'translate-x-0' : 'translate-x-full'
        }`}
      >
        <div className="flex items-center justify-between p-6 border-b border-slate-700/50">
          <div>
            <h2 className="text-xl font-bold text-white">Menu</h2>
            <p className="text-xs text-slate-400 mt-0.5">
              Navigate your workspace
            </p>
          </div>
          <button
            onClick={() => setIsOpen(false)}
            className="p-2 hover:bg-slate-800 rounded-lg transition-colors text-slate-300 hover:text-white"
            aria-label="Close menu"
          >
            <X size={24} />
          </button>
        </div>

        {/* Navigation Links */}
        <nav className="flex-1 p-4 overflow-y-auto">
          <NavLink
            to="/"
            onClick={() => setIsOpen(false)}
            className="flex items-center gap-3 p-4 rounded-xl hover:bg-slate-800/50 transition-all mb-2 text-slate-300 hover:text-white group"
            activeProps={{
              className:
                'flex items-center gap-3 p-4 rounded-xl bg-gradient-to-r from-cyan-600 to-blue-600 transition-all mb-2 text-white shadow-lg shadow-cyan-500/20',
            }}
          >
            <div className="p-2 rounded-lg bg-slate-800 group-hover:bg-slate-700 transition-colors">
              <Home size={20} />
            </div>
            <span className="font-medium">Home</span>
          </NavLink>
          <NavLink
            to="/subscriptions"
            onClick={() => setIsOpen(false)}
            className="flex items-center gap-3 p-4 rounded-xl hover:bg-slate-800/50 transition-all mb-2 text-slate-300 hover:text-white group"
            activeProps={{
              className:
                'flex items-center gap-3 p-4 rounded-xl bg-gradient-to-r from-cyan-600 to-blue-600 transition-all mb-2 text-white shadow-lg shadow-cyan-500/20',
            }}
          >
            <div className="p-2 rounded-lg bg-slate-800 group-hover:bg-slate-700 transition-colors">
              <Home size={20} />
            </div>
            <span className="font-medium">Subscriptions</span>
          </NavLink>
        </nav>

        <div className="p-4 border-t border-slate-700/50">
          <div className="flex items-center gap-3 p-3 rounded-lg bg-slate-800/50">
            <Circle
              size={10}
              className={`${
                isConnected
                  ? 'fill-green-500 text-green-500'
                  : 'fill-red-500 text-red-500'
              }`}
            />
            <div className="flex-1">
              <p className="text-xs font-medium text-slate-300">Status</p>
              <p className="text-[10px] text-slate-500">
                {isConnected ? 'Connected to server' : 'Disconnected'}
              </p>
            </div>
          </div>
        </div>
      </aside>
    </>
  )
}
