<script lang="ts">
  import { fileUrl, type ListedChild } from '../lib/api'
  import MediaTile from './MediaTile.svelte'

  const STEP = 30

  let { items, rel }: { items: ListedChild[]; rel: string } = $props()

  let visibleCount = $state(STEP)
  let sentinel: HTMLDivElement | undefined = $state()

  const visible = $derived(items.slice(0, visibleCount))

  $effect(() => {
    items
    visibleCount = STEP
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
</script>

<div
  class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5"
>
  {#each visible as item (item.name)}
    <MediaTile {item} url={fileUrl(rel, item.name)} />
  {/each}
</div>
{#if visibleCount < items.length}
  <div bind:this={sentinel} class="h-px" aria-hidden="true"></div>
{/if}