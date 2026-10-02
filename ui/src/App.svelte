<script lang="ts">
  import Router from 'svelte-spa-router'
  import { routes } from './routes'
</script>

<div class="navbar bg-primary text-primary-content">
  <div class="flex-1 px-2">
    <a class="text-xl font-bold" href="#/">Album</a>
  </div>
  <div class="flex-none">
    <ul class="menu menu-horizontal px-1">
      <li><a href="#/">Home</a></li>
    </ul>
  </div>
</div>

<main class="container mx-auto p-4">
  <Router {routes} />
</main>