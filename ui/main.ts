import { mount } from 'svelte';
import App from './App.svelte';
import './theme.css';
import './style.css';
import { applyTheme, readThemePreference } from './theme';

applyTheme(readThemePreference());
mount(App, { target: document.getElementById('app')! });
