<script lang="ts">
  import { replace } from 'svelte-spa-router'

  const DESTINATION = '/album'

  $effect(() => {
    replace(DESTINATION)
  })
</script>
