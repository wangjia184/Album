<script lang="ts">
  import { push } from 'svelte-spa-router'
  import Stage from '../components/Stage.svelte'
  import FilmStrip from '../components/FilmStrip.svelte'
  import CoverFlow from '../components/CoverFlow.svelte'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref } from '../lib/path'

  let current = $state<string | null>(null)
  // Default = orientation read ONCE at load (no resize re-check); the floating
  // toggle pins a manual choice until reload. Cover also requires a
  // Web Worker + OffscreenCanvas environment (worker-backed canvas paint).
  let manual: 'cover' | 'strip' | null = $state(null)

  const canCover =
    typeof window !== 'undefined' &&
    typeof Worker !== 'undefined' &&
    typeof OffscreenCanvas !== 'undefined' &&
    typeof HTMLCanvasElement !== 'undefined' &&
    typeof HTMLCanvasElement.prototype.transferControlToOffscreen === 'function'

  // Synchronous initial value: a false→true flip in onMount would flash strip
  // mode (and fire its queue fetch) before switching to cover on load.
  let landscape = $state(
    typeof window !== 'undefined' && window.matchMedia('(orientation: landscape)').matches,
  )

  const mode = $derived.by(() => {
    if (manual === 'strip') return 'strip'
    if (manual === 'cover') return canCover ? 'cover' : 'strip'
    return landscape && canCover ? 'cover' : 'strip'
  })

  function open(path: string): void {
    const { dir, file } = splitMediaPath(path)
    push(mediaHref(dir, file))
  }
</script>

<section class="relative flex min-h-0 flex-1 flex-col gap-3">
  <div
    class="join absolute right-2 top-2 z-20 rounded-lg bg-base-100/60 p-1 backdrop-blur"
  >
    <button
      type="button"
      class="join-item btn btn-xs {mode === 'strip' ? 'btn-active' : ''}"
      onclick={() => {
        manual = 'strip'
      }}>画框</button
    >
    <button
      type="button"
      class="join-item btn btn-xs {mode === 'cover' ? 'btn-active' : ''}"
      disabled={!canCover}
      title={canCover ? undefined : '当前环境不支持 Web Worker'}
      onclick={() => {
        if (canCover) manual = 'cover'
      }}>封面</button
    >
  </div>
  {#if mode === 'cover'}
    <CoverFlow />
  {:else}
    <Stage {current} onopen={open} />
    <FilmStrip oncurrent={(path) => { current = path }} />
  {/if}
</section>