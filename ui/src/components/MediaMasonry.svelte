<script lang="ts">
  import { Frame, RegularMasonryGrid } from '@masonry-grid/svelte'
  import { fileUrl, type ListedChild } from '../lib/api'
  import MediaTile from './MediaTile.svelte'

  const STEP = 30
  const GAP = 12
  const DESKTOP_TILE_TARGET = 240

  let {
    items,
    rel,
    onselect,
  }: {
    items: ListedChild[]
    rel: string
    onselect: (index: number) => void
  } = $props()

  let visibleCount = $state(STEP)
  let sentinel: HTMLDivElement | undefined = $state()
  let containerEl: HTMLDivElement | undefined = $state()
  let containerWidth = $state(0)
  let aspects = $state<Record<string, { w: number; h: number }>>({})

  const visible = $derived(items.slice(0, visibleCount))

  const columnCount = $derived.by(() => {
    const width = containerWidth
    if (width <= 0) return 2
    if (width < 640) return 2
    if (width < 1024) return 3
    return Math.min(5, Math.max(4, Math.round(width / DESKTOP_TILE_TARGET)))
  })

  const frameWidth = $derived.by(() => {
    const width = containerWidth
    if (width <= 0) return '50%'
    const usable = width - (columnCount - 1) * GAP
    return `${Math.max(1, Math.floor(usable / columnCount))}px`
  })

  function aspectFor(name: string, kind: ListedChild['kind']): {
    w: number
    h: number
  } {
    if (kind === 'video') return { w: 16, h: 9 }
    return aspects[name] ?? { w: 4, h: 3 }
  }

  function handleLoad(name: string, width: number, height: number): void {
    if (width <= 0 || height <= 0) return
    const current = aspects[name]
    if (current && current.w === width && current.h === height) return
    aspects[name] = { w: width, h: height }
  }

  $effect(() => {
    items
    visibleCount = STEP
    aspects = {}
  })

  $effect(() => {
    const el = sentinel
    const count = visibleCount
    if (el === undefined) return
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return
      if (count < items.length) {
        visibleCount = Math.min(visibleCount + STEP, items.length)
      }
    })
    observer.observe(el)
    return () => observer.disconnect()
  })

  $effect(() => {
    const el = containerEl
    if (el === undefined) return
    const measure = (): void => {
      containerWidth = el.clientWidth
    }
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(el)
    return () => observer.disconnect()
  })
</script>

<div bind:this={containerEl}>
  <RegularMasonryGrid {frameWidth} gap={GAP}>
    {#each visible as item, index (item.name)}
      {@const aspect = aspectFor(item.name, item.kind)}
      <Frame width={aspect.w} height={aspect.h}>
        <MediaTile
          {item}
          url={fileUrl(rel, item.name)}
          onselect={() => onselect(index)}
          onload={(w, h) => handleLoad(item.name, w, h)}
        />
      </Frame>
    {/each}
  </RegularMasonryGrid>
</div>
{#if visibleCount < items.length}
  <div bind:this={sentinel} class="h-px" aria-hidden="true"></div>
{/if}