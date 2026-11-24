<script lang="ts">
  import { onMount } from 'svelte';
  import Header from './components/Header.svelte';
  import ToastContainer from './components/ToastContainer.svelte';
  import ConverterPage from './pages/ConverterPage.svelte';
  import SummarizerPage from './pages/SummarizerPage.svelte';
  import MdToPdfPage from './pages/MdToPdfPage.svelte';
  import { fetchAppConfig, type AppConfigResponse } from './lib/api';
  import { i18n } from './lib/i18n.svelte';
  import { router } from './lib/router.svelte';
  import { toasts } from './lib/toast.svelte';

  const fallbackConfig: AppConfigResponse = {
    appName: 'Distilla',
    models: [{ id: 'gpt-4o', label: 'GPT-4o' }],
    defaultModel: 'gpt-4o',
    maxUploadSizeMb: 1024,
    mediaMaxOutputSizeMb: 100,
    supportedExtensions: [
      '.pdf', '.txt', '.md', '.docx', '.doc', '.pptx', '.ppt', '.xlsx', '.xls',
      '.png', '.jpg', '.jpeg', '.webp', '.gif',
      '.mp3', '.wav', '.ogg', '.m4a', '.aac', '.opus', '.flac', '.oga', '.weba',
      '.mp4', '.mov', '.avi', '.mkv', '.webm', '.3gp', '.m4v', '.mpeg', '.mpg', '.wmv', '.flv',
    ],
    formats: [],
    capabilities: { ai: true, ffmpeg: true, office: true, pdf: true },
  };

  let appConfig = $state<AppConfigResponse>(fallbackConfig);

  onMount(async () => {
    try {
      appConfig = await fetchAppConfig();
      document.title = `${appConfig.appName} — ${i18n.t('app.tagline')}`;
    } catch {
      toasts.show(i18n.t('toast.serverUnreachable'), 'error');
    }
  });
</script>

<div class="flex min-h-screen flex-col">
  <Header appName={appConfig.appName} />

  <main class="flex-1 px-4 py-8">
    {#if router.path === '/summarizer'}
      <SummarizerPage config={appConfig} />
    {:else if router.path === '/md-to-pdf'}
      <MdToPdfPage config={appConfig} />
    {:else}
      <ConverterPage config={appConfig} />
    {/if}
  </main>

  <ToastContainer />
</div>
