export type RoutePath = '/' | '/summarizer' | '/md-to-pdf';

function normalizePath(raw: string): RoutePath {
  if (raw.startsWith('/summarizer')) return '/summarizer';
  if (raw.startsWith('/md-to-pdf')) return '/md-to-pdf';
  return '/';
}

class RouterStore {
  path = $state<RoutePath>(
    typeof window !== 'undefined' ? normalizePath(window.location.pathname) : '/',
  );

  constructor() {
    if (typeof window !== 'undefined') {
      window.addEventListener('popstate', () => {
        this.path = normalizePath(window.location.pathname);
      });
    }
  }

  navigate(target: RoutePath) {
    if (this.path === target) return;
    if (typeof window !== 'undefined') {
      window.history.pushState(null, '', target);
    }
    this.path = target;
  }
}

export const router = new RouterStore();
