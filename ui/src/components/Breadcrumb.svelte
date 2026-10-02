<script lang="ts">
  let { path }: { path: string } = $props()

  const segments = $derived(
    path.split('/').filter((segment) => segment.length > 0),
  )

  function hrefFor(index: number): string {
    const prefix = segments
      .slice(0, index + 1)
      .map(encodeURIComponent)
      .join('/')
    return `#/album/${prefix}`
  }
</script>

<div class="breadcrumbs text-sm">
  <ul>
    <li><a href="#/album">相册</a></li>
    {#each segments as segment, index (index)}
      {#if index === segments.length - 1}
        <li>{segment}</li>
      {:else}
        <li><a href={hrefFor(index)}>{segment}</a></li>
      {/if}
    {/each}
  </ul>
</div>