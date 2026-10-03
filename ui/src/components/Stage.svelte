<script lang="ts">
  import { fileUrl } from '../lib/api'

  let { current, onopen }: { current: string | null; onopen: (path: string) => void } = $props()

  const FADE_MS = 800
  const FX_MS = 400
  const EFFECTS = ['kenburns', 'flip', 'wipe'] as const
  type FancyFx = (typeof EFFECTS)[number]
  type TransitionKind = FancyFx | 'fade'

  let shownSrc = $state<string | null>(null)
  let shownDims = $state<{ w: number; h: number } | null>(null)
  let visible = $state(false)
  let failed = $state(false)
  let incoming = $state<{ src: string; fx: FancyFx } | null>(null)
  let lastTransition = $state<TransitionKind | ''>('')
  let seq = 0

  function arSame(a: { w: number; h: number }, b: { w: number; h: number }): boolean {
    const r1 = a.w / a.h
    const r2 = b.w / b.h
    return Math.abs(r1 - r2) / r1 < 0.01
  }

  function pickFx(): FancyFx {
    return EFFECTS[Math.floor(Math.random() * EFFECTS.length)]
  }

  $effect(() => {
    const url = current === null ? null : fileUrl('', current)
    seq += 1
    const mySeq = seq
    failed = false
    incoming = null
    if (url === null) {
      shownSrc = null
      shownDims = null
      visible = false
      lastTransition = ''
      return
    }
    const pre = new Image()
    pre.onload = () => {
      if (mySeq !== seq) return
      const dims = { w: pre.naturalWidth, h: pre.naturalHeight }
      if (shownSrc === null || shownDims === null) {
        shownSrc = url
        shownDims = dims
        visible = true
        return
      }
      // TODO(follow-up) retired: same-aspect fancy transition implemented below.
      if (arSame(shownDims, dims)) {
        const fx = pickFx()
        lastTransition = fx
        incoming = { src: url, fx }
        setTimeout(() => {
          if (mySeq !== seq) return
          shownSrc = url
          shownDims = dims
          incoming = null
          visible = true
        }, FX_MS + 50)
      } else if (!visible) {
        // Interrupted mid fade-out: skip the out phase, fade the new photo in.
        lastTransition = 'fade'
        shownSrc = url
        shownDims = dims
        visible = true
      } else {
        lastTransition = 'fade'
        visible = false
        setTimeout(() => {
          if (mySeq !== seq) return
          shownSrc = url
          shownDims = dims
          visible = true
        }, FADE_MS)
      }
    }
    pre.onerror = () => {
      if (mySeq !== seq) return
      failed = true
      incoming = null
    }
    pre.src = url
  })

  function openCurrent(): void {
    if (current !== null && !failed) onopen(current)
  }
</script>

<div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden">
  {#if failed}
    <p class="text-sm opacity-50">图片加载失败</p>
  {:else if shownSrc !== null}
    <button
      type="button"
      class="flex h-full w-full cursor-pointer items-center justify-center"
      aria-label="打开这张照片"
      onclick={openCurrent}
    >
      <div
        class="bg-white p-3 ring-1 ring-base-300 shadow-[0_12px_40px_rgba(0,0,0,0.55)]"
        data-transition={lastTransition}
      >
        <div class="stage-fx-host relative">
          <img
            data-testid="stage-img"
            src={shownSrc}
            alt=""
            class="block max-h-[calc(100dvh-14rem)] max-w-[calc(100vw-6rem)] object-contain transition-opacity duration-[800ms]"
            style:opacity={visible ? '1' : '0'}
          />
          {#if incoming}
            <img
              data-testid="stage-incoming"
              src={incoming.src}
              alt=""
              class="fx-{incoming.fx} absolute inset-0 h-full w-full"
            />
          {/if}
        </div>
      </div>
    </button>
  {/if}
</div>