<script lang="ts">
  const PLACEHOLDER_ITEMS = [
    { id: 1 },
    { id: 2 },
    { id: 3 },
    { id: 4 },
    { id: 5 },
  ]
</script>

<section class="flex flex-col gap-4">
  <div class="card bg-base-200">
    <div class="card-body">
      <h1 class="card-title">随机轮播（占位）</h1>
      <p class="text-sm opacity-70">精选照片自动轮播，后续接入真实媒体。</p>
    </div>
  </div>

  <div class="carousel carousel-center w-full gap-4 rounded-box bg-base-200 p-4">
    {#each PLACEHOLDER_ITEMS as item (item.id)}
      <div class="carousel-item flex h-64 w-96 flex-col items-center justify-center rounded-box bg-base-300">
        <span class="text-4xl opacity-40">{item.id}</span>
      </div>
    {/each}
  </div>
</section>