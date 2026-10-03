<script lang="ts">
  import { albumHref, prefixHref, toSegments } from '../lib/path'

  let { path }: { path: string } = $props()

  const segments = $derived(toSegments(path))
</script>

<div class="breadcrumbs text-sm">
  <ul>
    <li><a href={albumHref([])}>相册</a></li>
    {#each segments as segment, index (index)}
      <li><a href={prefixHref(segments, index)}>{segment}</a></li>
    {/each}
  </ul>
</div>