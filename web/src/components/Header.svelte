<script lang="ts">
  import { i18n } from '../lib/i18n.svelte';
  import { router, type RoutePath } from '../lib/router.svelte';
  import { theme } from '../lib/theme.svelte';

  let { appName = 'Distilla' }: { appName?: string } = $props();
  let mobileMenuOpen = $state(false);

  const navItems: { path: RoutePath; labelKey: string }[] = [
    { path: '/', labelKey: 'nav.converter' },
    { path: '/summarizer', labelKey: 'nav.summarizer' },
    { path: '/md-to-pdf', labelKey: 'nav.editor' },
  ];

  function go(path: RoutePath, e: MouseEvent) {
    e.preventDefault();
    router.navigate(path);
    mobileMenuOpen = false;
  }
</script>

<header class="surface-header sticky top-0 z-50">
  <div class="mx-auto flex max-w-7xl items-center justify-between px-4 py-3.5">
    <div class="flex items-center gap-6">
      <a href="/" onclick={(e) => go('/', e)} class="flex items-center gap-3">
        <img
          src="/favicon.svg"
          alt={appName}
          class="h-10 w-10 rounded-xl shadow-lg shadow-indigo-500/25"
        />
        <div class="hidden sm:block">
          <h1 class="bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-xl font-bold text-transparent">
            {appName}
          </h1>
          <p class="text-[11px] text-dim">{i18n.t('app.tagline')}</p>
        </div>
      </a>

      <nav class="surface-elevated hidden items-center gap-1 rounded-xl p-1 md:flex">
        {#each navItems as item}
          <a
            href={item.path}
            onclick={(e) => go(item.path, e)}
            class="rounded-lg px-3.5 py-1.5 text-sm font-medium transition-all {router.path === item.path
              ? 'bg-indigo-600 text-white shadow-sm'
              : 'text-sub hover:text-main'}"
          >
            {i18n.t(item.labelKey)}
          </a>
        {/each}
      </nav>
    </div>

    <div class="flex items-center gap-2.5">
      <!-- Language Switcher -->
      <div class="surface-elevated flex items-center rounded-lg p-1">
        <button
          type="button"
          onclick={() => i18n.setLang('tr')}
          class="rounded-md px-2.5 py-1 text-xs font-semibold transition-all {i18n.lang === 'tr'
            ? 'bg-indigo-600 text-white'
            : 'text-sub hover:text-main'}"
        >
          TR
        </button>
        <button
          type="button"
          onclick={() => i18n.setLang('en')}
          class="rounded-md px-2.5 py-1 text-xs font-semibold transition-all {i18n.lang === 'en'
            ? 'bg-indigo-600 text-white'
            : 'text-sub hover:text-main'}"
        >
          EN
        </button>
      </div>

      <!-- Theme Toggle -->
      <button
        type="button"
        onclick={() => theme.toggle()}
        class="btn-secondary flex h-9 w-9 items-center justify-center"
        title={theme.current === 'dark' ? i18n.t('theme.light') : i18n.t('theme.dark')}
        aria-label={theme.current === 'dark' ? i18n.t('theme.light') : i18n.t('theme.dark')}
      >
        {#if theme.current === 'dark'}
          <svg class="h-4 w-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"
            />
          </svg>
        {:else}
          <svg class="h-4 w-4 text-indigo-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"
            />
          </svg>
        {/if}
      </button>

      <!-- Mobile Menu Toggle -->
      <button
        type="button"
        onclick={() => (mobileMenuOpen = !mobileMenuOpen)}
        class="btn-secondary flex h-9 w-9 items-center justify-center md:hidden"
        aria-label="Menu"
      >
        <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16m-7 6h7" />
        </svg>
      </button>
    </div>
  </div>

  {#if mobileMenuOpen}
    <div class="surface-header border-t px-4 py-3 md:hidden">
      <div class="flex flex-col gap-1.5">
        {#each navItems as item}
          <a
            href={item.path}
            onclick={(e) => go(item.path, e)}
            class="rounded-lg px-3.5 py-2 text-sm font-medium transition-all {router.path === item.path
              ? 'bg-indigo-600 text-white'
              : 'text-sub hover:text-main'}"
          >
            {i18n.t(item.labelKey)}
          </a>
        {/each}
      </div>
    </div>
  {/if}
</header>
