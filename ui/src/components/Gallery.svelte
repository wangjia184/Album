<script lang="ts">
  import type { ListedChild } from '../lib/api'
  import { columnCount, frameWidthPx } from '../lib/grid'
  import FolderGrid from './FolderGrid.svelte'
  import MediaMasonry from './MediaMasonry.svelte'

  let {
    dirs,
    media,
    rel,
    onselect,
  }: {
    dirs: ListedChild[]
    media: ListedChild[]
    rel: string
    onselect: (index: number) => void
  } = $props()

  let containerEl: HTMLDivElement | undefined = $state()
  let width = $state(0)

  const cols = $derived(columnCount(width))
  const frameWidth = $derived(frameWidthPx(width, cols))

  $effect(() => {
    const el = containerEl
    if (el === undefined) return
    const measure = (): void => {
      width = el.clientWidth
    }
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(el)
    return () => observer.disconnect()
  })
</script>

<div bind:this={containerEl} class="flex flex-col gap-4">
  {#if width > 0}
    {#if dirs.length > 0}
      <FolderGrid {dirs} {rel} {cols} {frameWidth} />
    {/if}
    {#if media.length > 0}
      <MediaMasonry items={media} {rel} {frameWidth} {onselect} />
    {/if}
  {/if}
</div>