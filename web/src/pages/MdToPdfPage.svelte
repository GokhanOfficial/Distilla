<script lang="ts">
  import { generatePdfBlob, generateTitle, type AppConfigResponse } from '../lib/api';
  import { i18n } from '../lib/i18n.svelte';
  import { toasts } from '../lib/toast.svelte';
  import { copyText, downloadBlob, downloadMarkdownFile, simpleHash } from '../lib/utils';
  import MarkdownPreview from '../components/MarkdownPreview.svelte';

  let { config }: { config: AppConfigResponse } = $props();

  let markdown = $state('');
  let editorEl: HTMLTextAreaElement | null = null;
  let previewEl = $state<HTMLDivElement | null>(null);
  let syncingScroll = false;

  let cachedTitle = $state<string | null>(null);
  let cachedTitleHash = $state('');
  let exportingPdf = $state(false);

  const charCount = $derived(markdown.length);
  const wordCount = $derived(markdown.trim() ? markdown.trim().split(/\s+/).length : 0);

  function syncFromEditor() {
    if (syncingScroll || !editorEl || !previewEl) return;
    const maxEditor = editorEl.scrollHeight - editorEl.clientHeight;
    const maxPreview = previewEl.scrollHeight - previewEl.clientHeight;
    if (maxEditor <= 0 || maxPreview <= 0) return;

    syncingScroll = true;
    const ratio = editorEl.scrollTop / maxEditor;
    previewEl.scrollTop = ratio * maxPreview;
    requestAnimationFrame(() => {
      syncingScroll = false;
    });
  }

  function syncFromPreview() {
    if (syncingScroll || !editorEl || !previewEl) return;
    const maxEditor = editorEl.scrollHeight - editorEl.clientHeight;
    const maxPreview = previewEl.scrollHeight - previewEl.clientHeight;
    if (maxEditor <= 0 || maxPreview <= 0) return;

    syncingScroll = true;
    const ratio = previewEl.scrollTop / maxPreview;
    editorEl.scrollTop = ratio * maxEditor;
    requestAnimationFrame(() => {
      syncingScroll = false;
    });
  }

  async function ensureTitle(): Promise<string> {
    const hash = simpleHash(markdown);
    if (cachedTitle && cachedTitleHash === hash) return cachedTitle;
    if (!config.capabilities.ai) return 'document';
    toasts.show(i18n.t('toast.generatingTitle'), 'info', 2000);
    try {
      const title = await generateTitle(markdown);
      cachedTitle = title;
      cachedTitleHash = hash;
      return title;
    } catch {
      return 'document';
    }
  }

  async function handleCopy() {
    if (!markdown.trim()) {
      toasts.show(i18n.t('toast.emptyContent'), 'error');
      return;
    }
    if (await copyText(markdown)) {
      toasts.show(i18n.t('toast.copied'), 'success');
    }
  }

  async function handleDownloadMd() {
    if (!markdown.trim()) {
      toasts.show(i18n.t('toast.emptyContent'), 'error');
      return;
    }
    const title = await ensureTitle();
    downloadMarkdownFile(markdown, title);
    toasts.show(i18n.t('toast.downloadStarted'), 'success');
  }

  async function handleDownloadPdf() {
    if (!markdown.trim()) {
      toasts.show(i18n.t('toast.emptyContent'), 'error');
      return;
    }
    exportingPdf = true;
    try {
      const title = await ensureTitle();
      toasts.show(i18n.t('toast.generatingPdf'), 'info', 2500);
      const { blob, filename } = await generatePdfBlob(markdown, title);
      downloadBlob(blob, filename);
      toasts.show(i18n.t('toast.pdfReady'), 'success');
    } catch (err) {
      toasts.show(err instanceof Error ? err.message : 'PDF error', 'error');
    } finally {
      exportingPdf = false;
    }
  }
</script>

<div class="mx-auto max-w-7xl space-y-5">
  <div class="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
    <div>
      <h2 class="text-2xl font-bold tracking-tight text-main">{i18n.t('editor.title')}</h2>
      <p class="mt-1 text-sm text-sub">{i18n.t('editor.subtitle')}</p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <button type="button" onclick={handleCopy} class="btn-secondary px-3.5 py-2 text-xs">
        {i18n.t('actions.copy')}
      </button>
      <button type="button" onclick={handleDownloadMd} class="btn-secondary px-3.5 py-2 text-xs">
        {i18n.t('actions.downloadMd')}
      </button>
      <button
        type="button"
        disabled={exportingPdf}
        onclick={handleDownloadPdf}
        class="btn-primary px-4 py-2 text-xs"
      >
        {exportingPdf ? i18n.t('toast.generatingPdf') : i18n.t('actions.downloadPdf')}
      </button>
    </div>
  </div>

  <div class="grid grid-cols-1 gap-5 lg:grid-cols-2">
    <div class="surface-card overflow-hidden">
      <div class="flex items-center justify-between border-b border-[var(--border-subtle)] px-4 py-3">
        <span class="text-xs font-semibold text-sub">{i18n.t('editor.markdownTitle')}</span>
        <span class="text-xs text-dim">
          {charCount.toLocaleString()} {i18n.t('converter.chars')} · {wordCount.toLocaleString()} {i18n.t('converter.words')}
        </span>
      </div>
      <textarea
        bind:this={editorEl}
        bind:value={markdown}
        onscroll={syncFromEditor}
        placeholder={i18n.t('editor.placeholder')}
        class="h-[600px] w-full resize-none bg-transparent p-5 font-mono text-sm leading-relaxed text-main focus:outline-none"
      ></textarea>
    </div>

    <div class="surface-card overflow-hidden">
      <div class="border-b border-[var(--border-subtle)] px-4 py-3">
        <span class="text-xs font-semibold text-sub">{i18n.t('editor.previewTitle')}</span>
      </div>
      <MarkdownPreview
        {markdown}
        placeholder={i18n.t('converter.previewEmpty')}
        bind:containerEl={previewEl}
        onscroll={syncFromPreview}
        heightClass="h-[600px]"
      />
    </div>
  </div>
</div>
