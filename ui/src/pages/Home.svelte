<script lang="ts">
  import { push } from 'svelte-spa-router'
  import Stage from '../components/Stage.svelte'
  import FilmStrip from '../components/FilmStrip.svelte'
  import { splitMediaPath } from '../lib/pipeline'
  import { mediaHref } from '../lib/path'

  let current = $state<string | null>(null)

  function open(path: string): void {
    const { dir, file } = splitMediaPath(path)
    push(mediaHref(dir, file))
  }
</script>

<section class="flex min-h-0 flex-1 flex-col gap-3">
  <Stage {current} onopen={open} />
  <FilmStrip oncurrent={(path) => { current = path }} />
</section>