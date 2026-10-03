<script lang="ts">
  import { albumHref, prefixHref, toSegments } from '../lib/path'

  let { path, rootName }: { path: string; rootName: string } = $props()

  const segments = $derived(toSegments(path))
</script>

<div class="breadcrumbs text-sm">
  <ul>
    <li><a href={albumHref([])}>{rootName || '相册'}</a></li>
    {#each segments as segment, index (index)}
      <li><a href={prefixHref(segments, index)}>{segment}</a></li>
    {/each}
  </ul>
</div>