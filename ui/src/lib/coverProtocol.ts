/**
 * Main → cover worker. Main never fetches; it only sends index/motion facts.
 */
export type MainToWorker =
  | { type: 'init'; offset0: number; gen: number }
  | {
      type: 'rebase'
      displayBase: number
      queueDelta: number
      seq: number
      gen: number
    }
  | { type: 'attach'; key: number; canvas: OffscreenCanvas; gen: number }
  | { type: 'detach'; key: number; gen: number }
  | {
      type: 'resize'
      width: number
      height: number
      dpr: number
      gen: number
    }
  | { type: 'dispose' }

/**
 * Worker → main. Badges stay in the DOM; content arrives here.
 * `paths` is the only data channel for paths (badge + center open).
 */
export type WorkerToMain =
  | { type: 'status'; gen: number; status: 'ready' | 'empty' | 'error'; error?: string }
  | {
      type: 'paths'
      gen: number
      seq: number
      displayBase: number
      entries: { key: number; path: string | null }[]
    }
  | { type: 'log'; level: 'warn' | 'error'; message: string }
