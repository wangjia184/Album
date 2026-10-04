<script lang="ts">
  import { Frame, RegularMasonryGrid } from '@masonry-grid/svelte'
  import { fileUrl, type ListedChild } from '../lib/api'
  import { useVisibleSlice } from '../lib/useVisibleSlice.svelte'
  import MediaTile from './MediaTile.svelte'

  const GAP = 12

  let {
    items,
    rel,
    frameWidth,
    onselect,
  }: {
    items: ListedChild[]
    rel: string
    frameWidth: number
    onselect: (index: number) => void
  } = $props()

  let aspects = $state<Record<string, { w: number; h: number }>>({})

  const slice = useVisibleSlice(() => items, 30)
  const sentinelAction = slice.sentinel
  const visible = slice.visible

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
    aspects = {}
  })
</script>

{#if frameWidth > 0}
  <RegularMasonryGrid frameWidth="{frameWidth}px" gap={GAP}>
    {#each visible as item, index (item.name)}
      {@const aspect = aspectFor(item.name, item.kind)}
      <Frame
        width={aspect.w}
        height={aspect.h}
        style="transition: aspect-ratio 0.4s ease"
      >
        <MediaTile
          {item}
          url={fileUrl(rel, item.name, item.kind === 'image')}
          onselect={() => onselect(index)}
          onload={(w, h) => handleLoad(item.name, w, h)}
        />
      </Frame>
    {/each}
  </RegularMasonryGrid>
{/if}
{#if slice.hasMore}
  <div use:sentinelAction class="h-px" aria-hidden="true"></div>
{/if}