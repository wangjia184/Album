<script lang="ts">
</script>

<div class="navbar bg-primary text-primary-content sticky top-0 z-40 shadow">
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
        <ul class="menu dropdown-content z-50 mt-2 w-52 rounded-box bg-primary p-2 shadow-lg">
          <li><a href="#/">首页</a></li>
          <li><a href="#/album">相册</a></li>
        </ul>
      </details>
    </div>
    <div class="hidden gap-1 lg:flex">
      <ul class="menu menu-horizontal px-1">
        <li><a href="#/">首页</a></li>
        <li><a href="#/album">相册</a></li>
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