import '@fontsource-variable/newsreader';
import '@fontsource-variable/source-sans-3';
import { mount } from 'svelte';
import App from './App.svelte';
import './styles/tokens.css';
import { applyTheme, readTheme } from './lib/theme';

applyTheme(readTheme(localStorage), document.documentElement);

mount(App, { target: document.getElementById('app')! });
