import { mount } from 'svelte';
import App from './App.svelte';
import './theme.css';
import './style.css';
import { applyTheme, readThemePreference } from './theme';
import { installInputModality } from './input-modality';
import { installNativeChrome } from './native-chrome';
import { installFieldCommit } from './field-commit';

applyTheme(readThemePreference());
installInputModality();
installNativeChrome();
installFieldCommit();
mount(App, { target: document.getElementById('app')! });
