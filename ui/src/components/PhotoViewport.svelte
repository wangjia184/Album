<script lang="ts">
  import ZoomToolbar from './ZoomToolbar.svelte'

  let { url, alt }: { url: string; alt: string } = $props()

  const MIN_SCALE = 0.1
  const MAX_SCALE = 4
  const STEP = 1.25

  let viewportEl: HTMLDivElement | undefined = $state()
  let imgEl: HTMLImageElement | undefined = $state()
  let viewportW = $state(0)
  let viewportH = $state(0)
  let natural = $state({ w: 0, h: 0 })
  let scale = $state(1)
  let offsetX = $state(0)
  let offsetY = $state(0)
  let dragging = $state(false)
  let dragStart = $state({ x: 0, y: 0, ox: 0, oy: 0 })

  const autofitScale = $derived.by(() => {
    if (natural.w <= 0 || natural.h <= 0 || viewportW <= 0 || viewportH <= 0) return 1
    return Math.min(viewportW / natural.w, viewportH / natural.h, 1)
  })

  const maxScale = $derived(
    Math.min(MAX_SCALE, natural.w > 0 ? Math.max(autofitScale, 1) : MAX_SCALE),
  )

  const exceeds = $derived.by(() => {
    if (natural.w <= 0) return false
    return natural.w * scale > viewportW + 1 || natural.h * scale > viewportH + 1
  })

  function clampOffsets(): void {
    const halfW = Math.max(0, (natural.w * scale - viewportW) / 2)
    const halfH = Math.max(0, (natural.h * scale - viewportH) / 2)
    offsetX = Math.max(-halfW, Math.min(halfW, offsetX))
    offsetY = Math.max(-halfH, Math.min(halfH, offsetY))
  }

  function autofit(): void {
    scale = autofitScale
    offsetX = 0
    offsetY = 0
  }

  function zoomBy(factor: number): void {
    const next = Math.min(maxScale, Math.max(MIN_SCALE, scale * factor))
    if (next === scale) return
    const ratio = next / scale
    offsetX *= ratio
    offsetY *= ratio
    scale = next
    clampOffsets()
  }

  function oneOne(): void {
    scale = Math.min(maxScale, Math.max(MIN_SCALE, 1))
    clampOffsets()
  }

  function handleLoad(event: Event): void {
    const img = event.currentTarget as HTMLImageElement
    natural = { w: img.naturalWidth, h: img.naturalHeight }
    autofit()
  }

  function handlePointerDown(event: PointerEvent): void {
    if (!exceeds) return
    dragging = true
    dragStart = { x: event.clientX, y: event.clientY, ox: offsetX, oy: offsetY }
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!dragging) return
    offsetX = dragStart.ox + (event.clientX - dragStart.x)
    offsetY = dragStart.oy + (event.clientY - dragStart.y)
    clampOffsets()
  }

  function handlePointerUp(event: PointerEvent): void {
    if (!dragging) return
    dragging = false
    ;(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId)
  }

  $effect(() => {
    void url
    natural = { w: 0, h: 0 }
    scale = 1
    offsetX = 0
    offsetY = 0
    dragging = false
  })

  $effect(() => {
    const el = viewportEl
    if (el === undefined) return
    const measure = (): void => {
      viewportW = el.clientWidth
      viewportH = el.clientHeight
    }
    measure()
    const observer = new ResizeObserver(() => {
      measure()
      clampOffsets()
    })
    observer.observe(el)
    return () => observer.disconnect()
  })

  $effect(() => {
    const img = imgEl
    if (img?.complete && img.naturalWidth > 0) {
      natural = { w: img.naturalWidth, h: img.naturalHeight }
      autofit()
    }
  })
</script>

<div class="flex h-full min-h-0 w-full flex-col">
  <div
    bind:this={viewportEl}
    class="relative min-h-0 flex-1 overflow-hidden bg-base-300 {exceeds
      ? dragging
        ? 'cursor-grabbing'
        : 'cursor-grab'
      : 'cursor-default'}"
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
    role="presentation"
  >
    <img
      bind:this={imgEl}
      src={url}
      {alt}
      draggable="false"
      class="absolute left-1/2 top-1/2 max-w-none select-none"
      style:transform="translate(-50%, -50%) translate({offsetX}px, {offsetY}px) scale({scale})"
      onload={handleLoad}
    />
  </div>
  <div class="shrink-0 border-t border-base-300 bg-base-100 p-2">
    <ZoomToolbar
      onzoomin={() => zoomBy(STEP)}
      onzoomout={() => zoomBy(1 / STEP)}
      ononeone={oneOne}
      onautofit={autofit}
    />
  </div>
</div>