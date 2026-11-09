import { listen } from '@tauri-apps/api/event'
import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react'
import { api } from '../shared/api'
import type { Bootstrap, Settings } from '../shared/types'

interface Store {
  boot: Bootstrap
  /** 백엔드 상태가 바뀔 때마다 올라가는 숫자. 페이지가 다시 불러올 때 의존성으로 쓴다. */
  tick: number
  reload: () => Promise<void>
  save: (patch: Partial<Settings>) => Promise<Settings>
}

const Ctx = createContext<Store | null>(null)

export function StoreProvider({ children, fallback }: { children: ReactNode; fallback: ReactNode }) {
  const [boot, setBoot] = useState<Bootstrap | null>(null)
  const [tick, setTick] = useState(0)

  const reload = useCallback(async () => setBoot(await api.bootstrap()), [])

  useEffect(() => {
    reload()
    const un = listen('ppok:changed', () => {
      setTick((t) => t + 1)
      reload()
    })
    return () => {
      un.then((f) => f())
    }
  }, [reload])

  const save = useCallback(
    async (patch: Partial<Settings>) => {
      if (!boot) throw new Error('not ready')
      const saved = await api.saveSettings({ ...boot.settings, ...patch })
      setBoot((b) => (b ? { ...b, settings: saved } : b))
      return saved
    },
    [boot],
  )

  if (!boot) return <>{fallback}</>
  return <Ctx.Provider value={{ boot, tick, reload, save }}>{children}</Ctx.Provider>
}

export function useStore() {
  const s = useContext(Ctx)
  if (!s) throw new Error('StoreProvider missing')
  return s
}
