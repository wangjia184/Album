<script lang="ts">
  import { push, router } from 'svelte-spa-router'
  import Stage from '../components/Stage.svelte'
  import FilmStrip from '../components/FilmStrip.svelte'
  import CoverFlow from '../components/CoverFlow.svelte'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref } from '../lib/path'

  let current = $state<string | null>(null)

  // ?mode=cover selects the cover-flow placeholder; anything else (incl.
  // missing/invalid) keeps the default stage + film-strip mode.
  const mode = $derived.by(() => {
    const qs = router.querystring
    return new URLSearchParams(qs || '').get('mode')
  })
  const isCover = $derived(mode === 'cover')

  function open(path: string): void {
    const { dir, file } = splitMediaPath(path)
    push(mediaHref(dir, file))
  }
</script>

<section class="relative flex min-h-0 flex-1 flex-col gap-3">
  <div class="join absolute right-2 top-2 z-20 rounded-lg bg-base-100/60 p-1 backdrop-blur">
    <a href="#/" class="join-item btn btn-xs {isCover ? '' : 'btn-active'}">横条</a>
    <a
      href="#/?mode=cover"
      class="join-item btn btn-xs {isCover ? 'btn-active' : ''}">Cover</a
    >
  </div>
  {#if isCover}
    <CoverFlow />
  {:else}
    <Stage {current} onopen={open} />
    <FilmStrip oncurrent={(path) => { current = path }} />
  {/if}
</section>