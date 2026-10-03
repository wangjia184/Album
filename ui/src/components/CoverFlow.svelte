<script lang="ts">
  // TODO(coverflow): independent pipeline (fetchQueue + own idle timer),
  // neighbor windows, click-to-lightbox, own frame/transitions.
  // Mode rule: no reuse with Stage/FilmStrip — only lib/api + pure helpers.
</script>

<div
  class="flex min-h-0 flex-1 items-center justify-center gap-4"
  data-testid="coverflow"
>
  <div
    class="h-40 w-28 shrink-0 rounded-lg bg-base-300/40 ring-1 ring-base-300/50"
    data-testid="cover-prev"
    aria-hidden="true"
  ></div>
  <div
    class="flex h-64 w-80 flex-col items-center justify-center gap-2 rounded-lg bg-base-300/60 ring-1 ring-base-300"
    data-testid="cover-center"
  >
    <span class="text-3xl opacity-40">▢</span>
    <span class="text-xs opacity-50">CoverFlow 占位</span>
  </div>
  <div
    class="h-40 w-28 shrink-0 rounded-lg bg-base-300/40 ring-1 ring-base-300/50"
    data-testid="cover-next"
    aria-hidden="true"
  ></div>
</div>