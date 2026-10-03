<script lang="ts">
  const FIELDS = [
    '拍摄时间',
    '相机',
    '光圈',
    '快门',
    'ISO',
    '焦距',
    'GPS',
    '评分',
    '色彩空间',
    '镜头',
  ] as const
</script>

<aside class="h-full w-72 shrink-0 overflow-y-auto border-l border-base-300 bg-base-100 p-4">
  <h2 class="mb-3 text-sm font-semibold opacity-70">媒体信息（EXIF）</h2>
  <p class="mb-4 text-xs opacity-50">接口暂不支持，以下为占位。</p>
  <dl class="space-y-2 text-sm">
    {#each FIELDS as field (field)}
      <div class="flex items-baseline justify-between gap-2">
        <dt class="shrink-0 opacity-70">{field}</dt>
        <dd class="text-right opacity-40">—</dd>
      </div>
    {/each}
  </dl>
</aside>