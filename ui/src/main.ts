import { mount } from 'svelte'
import './lib/icon-register'
import './app.css'
import App from './App.svelte'

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
