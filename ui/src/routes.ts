import Home from './pages/Home.svelte'
import Browse from './pages/Browse.svelte'

export const routes = {
  '/': Home,
  '/album': Browse,
  '/album/*': Browse,
}
