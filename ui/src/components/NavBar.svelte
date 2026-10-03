<script lang="ts">
  import { router } from 'svelte-spa-router'

  const route = $derived(router.location)

  const isHome = $derived(route === '/')
  const isAlbum = $derived(route === '/album' || (route ?? '').startsWith('/album/'))
</script>

<div class="navbar sticky top-0 z-40 border-b border-base-300 bg-base-100/50 text-base-content backdrop-blur-md shadow-sm">
  <div class="navbar-start">
    <div class="flex-none lg:hidden">
      <details class="dropdown">
        <summary class="btn btn-square btn-ghost" aria-label="菜单">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-5 w-5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            aria-hidden="true"
          >
            <path d="M4 6h16M4 12h16M4 18h16" />
          </svg>
        </summary>
        <ul class="menu dropdown-content z-50 mt-2 w-52 rounded-box bg-base-100 p-2 text-lg text-base-content shadow-lg">
          <li><a href="#/" class={isHome ? 'bg-primary/20 text-primary font-medium' : ''}><span aria-hidden="true">✨</span>流光</a></li>
          <li><a href="#/album" class={isAlbum ? 'bg-primary/20 text-primary font-medium' : ''}><span aria-hidden="true">🖼️</span>相册</a></li>
        </ul>
      </details>
    </div>
    <div class="hidden gap-1 lg:flex">
      <ul class="menu menu-horizontal gap-1 px-1 text-lg">
        <li><a href="#/" class={isHome ? 'bg-primary/20 text-primary font-medium rounded-lg' : ''}><span aria-hidden="true">✨</span>流光</a></li>
        <li><a href="#/album" class={isAlbum ? 'bg-primary/20 text-primary font-medium rounded-lg' : ''}><span aria-hidden="true">🖼️</span>相册</a></li>
      </ul>
    </div>
  </div>

  <div class="navbar-center">
    <a class="text-xl font-bold" href="#/">Album</a>
  </div>

  <div class="navbar-end">
    <a
      href="https://beian.miit.gov.cn"
      target="_blank"
      rel="noreferrer"
      class="px-2 text-xs opacity-80 hover:opacity-100"
    >
      备案编号：湘ICP备17022195号
    </a>
  </div>
</div>