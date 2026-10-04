import type { QueueItem } from './api'

/** Main → cover worker. Main never fetches; it only sends index/motion facts. */
export type MainToWorker =
  | { type: 'init'; offset0: number; limit: number; mid: number; gen: number }
  | { type: 'rebase'; displayBase: number; queueDelta: number; gen: number }
  | {
      type: 'attach'
      key: number
      canvas: OffscreenCanvas
      width: number
      height: number
      gen: number
    }
  | { type: 'detach'; key: number; gen: number }
  | { type: 'resize'; width: number; height: number; gen: number }
  | { type: 'dispose' }

/** Worker → main. Badges stay in the DOM; content arrives here. */
export type WorkerToMain =
  | { type: 'status'; status: 'ready' | 'empty' | 'error'; error?: string }
  | {
      type: 'paths'
      gen: number
      displayBase: number
      /** Parallel to keys displayBase±…; index 0 = displayBase - mid span */
      entries: { key: number; path: string | null }[]
      pathsByKey: Record<number, string>
    }
  | { type: 'log'; level: 'warn' | 'error'; message: string }

export type { QueueItem }