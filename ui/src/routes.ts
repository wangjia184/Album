import Browse from './pages/Browse.svelte'
import Home from './pages/Home.svelte'

export const routes = {
  '/': Home,
  '/album': Browse,
  '/album/*': Browse,
}