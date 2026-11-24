export type Theme = 'dark' | 'light';

const STORAGE_KEY = 'distilla-theme';

function getInitialTheme(): Theme {
  if (typeof localStorage !== 'undefined') {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'light' || saved === 'dark') return saved;
  }
  return 'dark';
}

class ThemeStore {
  current = $state<Theme>(getInitialTheme());

  constructor() {
    this.apply();
  }

  toggle() {
    this.current = this.current === 'dark' ? 'light' : 'dark';
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, this.current);
    }
    this.apply();
  }

  apply() {
    if (typeof document === 'undefined') return;
    document.documentElement.classList.toggle('light', this.current === 'light');
    document.documentElement.classList.toggle('dark', this.current === 'dark');
  }
}

export const theme = new ThemeStore();
