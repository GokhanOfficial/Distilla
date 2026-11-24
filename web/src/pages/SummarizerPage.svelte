<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    cancelJob,
    generatePdfBlob,
    startSummarizeJob,
    subscribeToJob,
    type AiStats,
    type AppConfigResponse,
  } from '../lib/api';
  import { i18n } from '../lib/i18n.svelte';
  import { toasts } from '../lib/toast.svelte';
  import { transfer } from '../lib/transfer.svelte';
  import { copyText, downloadBlob, downloadMarkdownFile, formatDuration } from '../lib/utils';
  import MarkdownPreview from '../components/MarkdownPreview.svelte';

  let { config }: { config: AppConfigResponse } = $props();

  let inputMarkdown = $state('');
  let summary = $state('');
  let generatedTitle = $state('summary');
  let selectedModel = $state('');

  let busy = $state(false);
  let cancelling = $state(false);
  let aiStats = $state<AiStats | null>(null);
  let exportingPdf = $state(false);

  let activeJobId: string | null = null;
  let unsubscribeSse: (() => void) | null = null;

  $effect(() => {
    if (!selectedModel && config.defaultSummarizeModel) {
      selectedModel = config.defaultSummarizeModel;
    }
  });

  onMount(() => {
    const incoming = transfer.consumeForSummarizer();
    if (incoming) {
      inputMarkdown = incoming;
    }
  });

  onDestroy(() => {
    if (unsubscribeSse) {
      unsubscribeSse();
      unsubscribeSse = null;
    }
  });

  async function handleSummarize() {
    if (!inputMarkdown.trim()) {
      toasts.show(i18n.t('toast.emptyContent'), 'error');
      return;
    }
    if (!config.capabilities.ai) {
      toasts.show(i18n.t('toast.aiNotConfigured'), 'error');
      return;
    }

    const modelToUse = selectedModel || config.defaultSummarizeModel;
    if (!modelToUse) {
      toasts.show(i18n.t('toast.noModelConfigured'), 'error');
      return;
    }

    if (unsubscribeSse) {
      unsubscribeSse();
      unsubscribeSse = null;
    }

    busy = true;
    cancelling = false;
    summary = '';
    aiStats = null;

    try {
      const { jobId } = await startSummarizeJob(inputMarkdown, modelToUse);
      activeJobId = jobId;

      unsubscribeSse = subscribeToJob('summarize', jobId, {
        onSnapshot(snap) {
          if (snap.markdown) summary = snap.markdown;
          if (snap.aiStats) aiStats = snap.aiStats;
          if (snap.generatedTitle) generatedTitle = snap.generatedTitle;
        },
        onChunk(chunk) {
          summary += chunk.delta;
          aiStats = chunk.aiStats;
        },
        onDone(done) {
          busy = false;
          cancelling = false;
          activeJobId = null;
          unsubscribeSse = null;

          if (done.status === 'completed') {
            summary = done.markdown;
            if (done.generatedTitle) generatedTitle = done.generatedTitle;
            toasts.show(i18n.t('toast.summaryComplete'), 'success');
          } else if (done.status === 'cancelled') {
            toasts.show(i18n.t('toast.summaryCancelled'), 'info');
          } else {
            toasts.show(done.error || i18n.t('progress.failed'), 'error');
          }
        },
        onError(err) {
          busy = false;
          cancelling = false;
          activeJobId = null;
          unsubscribeSse = null;
          toasts.show(err, 'error');
        },
      });
    } catch (err) {
      busy = false;
      cancelling = false;
      toasts.show(err instanceof Error ? err.message : i18n.t('progress.failed'), 'error');
    }
  }

  async function handleCancel() {
    if (!activeJobId) return;
    cancelling = true;
    try {
      await cancelJob('summarize', activeJobId);
    } catch {
      // ignore
    }
  }

  async function handleCopy() {
    if (!summary.trim()) return;
    if (await copyText(summary)) {
      toasts.show(i18n.t('toast.copied'), 'success');
    }
  }

  function handleDownloadMd() {
    if (!summary.trim()) return;
    downloadMarkdownFile(summary, generatedTitle || 'summary');
    toasts.show(i18n.t('toast.downloadStarted'), 'success');
  }

  async function handleDownloadPdf() {
    if (!summary.trim()) return;
    exportingPdf = true;
    try {
      toasts.show(i18n.t('toast.generatingPdf'), 'info', 2500);
      const { blob, filename } = await generatePdfBlob(summary, generatedTitle || 'summary');
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
      <h2 class="text-2xl font-bold tracking-tight text-main">{i18n.t('summarizer.title')}</h2>
      <p class="mt-1 text-sm text-sub">{i18n.t('summarizer.subtitle')}</p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <select
        bind:value={selectedModel}
        disabled={busy || config.summarizeModels.length === 0}
        class="surface-input rounded-lg px-3 py-2 text-xs"
      >
        {#if config.summarizeModels.length === 0}
          <option value="">{i18n.t('models.noneConfigured')}</option>
        {:else}
          {#each config.summarizeModels as model}
            <option value={model.id}>{model.label}</option>
          {/each}
        {/if}
      </select>

      {#if busy}
        <button
          type="button"
          disabled={cancelling}
          onclick={handleCancel}
          class="rounded-lg bg-red-600 px-4 py-2 text-xs font-semibold text-white transition-colors hover:bg-red-700 disabled:opacity-50"
        >
          {cancelling ? i18n.t('converter.cancelling') : i18n.t('converter.cancel')}
        </button>
      {:else}
        <button type="button" onclick={handleSummarize} class="btn-primary px-5 py-2 text-xs">
          {i18n.t('summarizer.start')}
        </button>
      {/if}
    </div>
  </div>

  {#if busy}
    <div class="surface-card flex items-center justify-between gap-4 px-4 py-3">
      <div class="flex items-center gap-3">
        <div class="h-4 w-4 animate-spin rounded-full border-2 border-indigo-500 border-t-transparent"></div>
        <span class="text-xs font-medium text-main">{i18n.t('summarizer.summarizing')}</span>
      </div>
      {#if aiStats}
        <span class="text-xs text-sub">
          {aiStats.chars.toLocaleString()} {i18n.t('converter.chars')} · {i18n.t('converter.elapsed')}: {formatDuration(aiStats.elapsedSeconds, i18n.lang)}
        </span>
      {/if}
    </div>
  {/if}

  <div class="grid grid-cols-1 gap-5 lg:grid-cols-2">
    <!-- Input Panel -->
    <div class="surface-card flex flex-col overflow-hidden">
      <div class="flex items-center justify-between border-b border-[var(--border-subtle)] px-4 py-3">
        <span class="text-xs font-semibold text-sub">{i18n.t('summarizer.inputTitle')}</span>
        <span class="text-xs text-dim">
          {inputMarkdown.length.toLocaleString()} {i18n.t('converter.chars')}
        </span>
      </div>
      <textarea
        bind:value={inputMarkdown}
        placeholder={i18n.t('summarizer.inputPlaceholder')}
        class="h-[560px] w-full resize-none bg-transparent p-5 font-mono text-sm leading-relaxed text-main focus:outline-none"
      ></textarea>
    </div>

    <!-- Output Panel -->
    <div class="surface-card flex flex-col overflow-hidden">
      <div class="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--border-subtle)] px-4 py-2.5">
        <span class="text-xs font-semibold text-sub">{i18n.t('summarizer.outputTitle')}</span>
        {#if summary.trim() && !busy}
          <div class="flex items-center gap-1.5">
            <button type="button" onclick={handleCopy} class="btn-secondary px-2.5 py-1 text-xs">
              {i18n.t('actions.copy')}
            </button>
            <button type="button" onclick={handleDownloadMd} class="btn-secondary px-2.5 py-1 text-xs">
              {i18n.t('actions.downloadMd')}
            </button>
            <button
              type="button"
              disabled={exportingPdf}
              onclick={handleDownloadPdf}
              class="btn-primary px-2.5 py-1 text-xs"
            >
              {exportingPdf ? '...' : i18n.t('actions.downloadPdf')}
            </button>
          </div>
        {/if}
      </div>
      <MarkdownPreview
        markdown={summary}
        placeholder={i18n.t('summarizer.outputPlaceholder')}
        heightClass="h-[560px]"
      />
    </div>
  </div>
</div>
