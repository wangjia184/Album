<script lang="ts">
  import { onMount } from 'svelte'
  import { push } from 'svelte-spa-router'
  import Stage from '../components/Stage.svelte'
  import FilmStrip from '../components/FilmStrip.svelte'
  import CoverFlow from '../components/CoverFlow.svelte'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref } from '../lib/path'

  let current = $state<string | null>(null)
  // Default = viewport orientation (landscape → cover, portrait → strip);
  // the floating toggle pins a manual choice until reload. No mode query.
  let manual: 'cover' | 'strip' | null = $state(null)
  let landscape = $state(false)

  const mode = $derived(manual ?? (landscape ? 'cover' : 'strip'))

  function open(path: string): void {
    const { dir, file } = splitMediaPath(path)
    push(mediaHref(dir, file))
  }

  onMount(() => {
    const mq = window.matchMedia('(orientation: landscape)')
    landscape = mq.matches
    const onChange = (e: MediaQueryListEvent): void => {
      landscape = e.matches
    }
    mq.addEventListener('change', onChange)
    return () => mq.removeEventListener('change', onChange)
  })
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
      onclick={() => {
        manual = 'cover'
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