<script lang="ts">
  import NavBar from './components/NavBar.svelte'
  import Router from 'svelte-spa-router'
  import { routes } from './routes'
</script>

<div class="flex min-h-screen flex-col">
  <NavBar />
  <main class="container mx-auto flex-1 p-4">
    <Router {routes} />
  </main>
</div>