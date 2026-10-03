import Redirect from './components/Redirect.svelte'
import Browse from './pages/Browse.svelte'

export const routes = {
  '/': Redirect,
  '/album': Browse,
  '/album/*': Browse,
}
