<script lang="ts">
  let { url, title }: { url: string; title: string } = $props()
</script>

<div class="flex h-full min-h-0 w-full items-center justify-center overflow-hidden bg-base-300 p-4">
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    src={url}
    aria-label={title}
    controls
    playsinline
    class="max-h-full max-w-full bg-black"
  ></video>
</div>