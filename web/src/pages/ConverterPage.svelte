<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    cancelJob,
    generatePdfBlob,
    generateTitle,
    startConversionUpload,
    subscribeToJob,
    type AiStats,
    type AppConfigResponse,
    type MediaProgress,
    type UploadProgress,
  } from '../lib/api';
  import { i18n } from '../lib/i18n.svelte';
  import { router } from '../lib/router.svelte';
  import { toasts } from '../lib/toast.svelte';
  import { transfer } from '../lib/transfer.svelte';
  import {
    copyText,
    downloadBlob,
    downloadMarkdownFile,
    formatDuration,
    formatFileSize,
    getFileCategory,
    simpleHash,
  } from '../lib/utils';
  import MarkdownPreview from '../components/MarkdownPreview.svelte';

  let { config }: { config: AppConfigResponse } = $props();

  let selectedFile = $state<File | null>(null);
  let selectedModel = $state('');
  let dragOver = $state(false);
  let fileInputEl: HTMLInputElement | null = null;

  // Active job state
  let busy = $state(false);
  let cancelling = $state(false);
  let stage = $state<'idle' | 'uploading' | 'processing'>('idle');
  let uploadProgress = $state<UploadProgress | null>(null);
  let jobProgress = $state(0);
  let jobMessageKey = $state('progress.queued');
  let mediaProgress = $state<MediaProgress | null>(null);
  let aiStats = $state<AiStats | null>(null);

  let activeUploadAbort: (() => void) | null = null;
  let activeJobId: string | null = null;
  let unsubscribeSse: (() => void) | null = null;

  // Result state
  let markdown = $state('');
  let hasResult = $state(false);
  let viewMode = $state<'markdown' | 'preview' | 'split'>('split');
  let cachedTitle = $state<string | null>(null);
  let cachedTitleHash = $state('');
  let exportingPdf = $state(false);

  $effect(() => {
    if (!selectedModel && config.defaultConvertModel) {
      selectedModel = config.defaultConvertModel;
    }
  });

  const badgeGroups = ['PDF', 'DOCX', 'PPTX', 'XLSX', 'TXT', 'MD', 'PNG', 'JPG', 'MP3', 'WAV', 'OGG', 'M4A', 'FLAC', 'MP4', 'MOV', 'MKV'];

  const acceptAttr = $derived(config.supportedExtensions.join(','));
  const charCount = $derived(markdown.length);
  const wordCount = $derived(markdown.trim() ? markdown.trim().split(/\s+/).length : 0);

  function cleanupActive() {
    if (activeUploadAbort) {
      activeUploadAbort();
      activeUploadAbort = null;
    }
    if (unsubscribeSse) {
      unsubscribeSse();
      unsubscribeSse = null;
    }
    activeJobId = null;
  }

  onDestroy(() => {
    cleanupActive();
  });

  function pickFile(file: File | undefined | null) {
    if (!file || busy) return;

    const ext = '.' + (file.name.toLowerCase().split('.').pop() || '');
    if (!config.supportedExtensions.includes(ext)) {
      toasts.show(i18n.t('toast.unsupportedFormat'), 'error');
      return;
    }
    if (file.size > config.maxUploadSizeMb * 1024 * 1024) {
      toasts.show(`${i18n.t('toast.fileTooLarge')} (${config.maxUploadSizeMb} MB)`, 'error');
      return;
    }

    selectedFile = file;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const file = e.dataTransfer?.files?.[0];
    pickFile(file);
  }

  function handleFileInput(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    pickFile(input.files?.[0]);
    input.value = '';
  }

  function clearFile() {
    if (busy) return;
    selectedFile = null;
  }

  function categoryBadgeColor(cat: string): string {
    switch (cat) {
      case 'pdf': return 'bg-red-500/15 text-red-400';
      case 'docx': return 'bg-blue-500/15 text-blue-400';
      case 'pptx': return 'bg-orange-500/15 text-orange-400';
      case 'xlsx': return 'bg-emerald-500/15 text-emerald-400';
      case 'audio': return 'bg-pink-500/15 text-pink-400';
      case 'video': return 'bg-indigo-500/15 text-indigo-400';
      case 'image': return 'bg-purple-500/15 text-purple-400';
      default: return 'bg-slate-500/15 text-slate-400';
    }
  }

  async function startConvert() {
    if (!selectedFile) {
      toasts.show(i18n.t('toast.noFile'), 'error');
      return;
    }
    if (!config.capabilities.ai) {
      toasts.show(i18n.t('toast.aiNotConfigured'), 'error');
      return;
    }

    const modelToUse = selectedModel || config.defaultConvertModel;
    if (!modelToUse) {
      toasts.show(i18n.t('toast.noModelConfigured'), 'error');
      return;
    }

    cleanupActive();
    busy = true;
    cancelling = false;
    stage = 'uploading';
    uploadProgress = { percent: 0, loaded: 0, total: selectedFile.size, averageSpeed: 0, etaSeconds: null };
    jobProgress = 0;
    jobMessageKey = 'progress.queued';
    mediaProgress = null;
    aiStats = null;
    markdown = '';
    cachedTitle = null;
    cachedTitleHash = '';

    try {
      const upload = startConversionUpload(selectedFile, modelToUse, (p) => {
        uploadProgress = p;
      });
      activeUploadAbort = upload.abort;

      const { jobId } = await upload.promise;
      activeUploadAbort = null;
      activeJobId = jobId;
      stage = 'processing';
      uploadProgress = null;

      unsubscribeSse = subscribeToJob('convert', jobId, {
        onSnapshot(snap) {
          jobProgress = snap.progress;
          jobMessageKey = snap.messageKey;
          mediaProgress = snap.mediaProgress ?? null;
          aiStats = snap.aiStats ?? null;
          if (snap.markdown) {
            markdown = snap.markdown;
            hasResult = true;
          }
        },
        onState(st) {
          jobProgress = st.progress;
          jobMessageKey = st.messageKey;
          mediaProgress = st.mediaProgress ?? null;
          if (st.aiStats) aiStats = st.aiStats;
        },
        onChunk(chunk) {
          markdown += chunk.delta;
          aiStats = chunk.aiStats;
          hasResult = true;
        },
        onDone(done) {
          busy = false;
          cancelling = false;
          stage = 'idle';
          activeJobId = null;
          unsubscribeSse = null;

          if (done.status === 'completed') {
            markdown = done.markdown;
            hasResult = true;
            toasts.show(i18n.t('toast.conversionComplete'), 'success');
          } else if (done.status === 'cancelled') {
            toasts.show(i18n.t('toast.conversionCancelled'), 'info');
          } else {
            toasts.show(done.error || i18n.t('progress.failed'), 'error');
          }
        },
        onError(err) {
          busy = false;
          cancelling = false;
          stage = 'idle';
          activeJobId = null;
          unsubscribeSse = null;
          toasts.show(err, 'error');
        },
      });
    } catch (err) {
      busy = false;
      cancelling = false;
      stage = 'idle';
      activeUploadAbort = null;
      if (err instanceof DOMException && err.name === 'AbortError') {
        toasts.show(i18n.t('toast.conversionCancelled'), 'info');
      } else {
        toasts.show(err instanceof Error ? err.message : i18n.t('progress.failed'), 'error');
      }
    }
  }

  async function handleCancel() {
    cancelling = true;
    if (activeUploadAbort) {
      activeUploadAbort();
      activeUploadAbort = null;
      return;
    }
    if (activeJobId) {
      try {
        await cancelJob('convert', activeJobId);
      } catch {
        // ignore network hiccup on cancel
      }
    }
  }

  async function ensureTitle(): Promise<string> {
    const hash = simpleHash(markdown);
    if (cachedTitle && cachedTitleHash === hash) return cachedTitle;
    toasts.show(i18n.t('toast.generatingTitle'), 'info', 2000);
    try {
      const title = await generateTitle(markdown, selectedModel || config.defaultConvertModel || undefined);
      cachedTitle = title;
      cachedTitleHash = hash;
      return title;
    } catch {
      const fallback = selectedFile?.name.replace(/\.[^/.]+$/, '') || 'document';
      return fallback;
    }
  }

  async function handleCopy() {
    if (!markdown.trim()) {
      toasts.show(i18n.t('toast.emptyContent'), 'error');
      return;
    }
    const ok = await copyText(markdown);
    if (ok) toasts.show(i18n.t('toast.copied'), 'success');
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

  function handleSendToSummarizer() {
    if (!markdown.trim()) {
      toasts.show(i18n.t('toast.emptyContent'), 'error');
      return;
    }
    transfer.sendToSummarizer(markdown);
    router.navigate('/summarizer');
  }
</script>

<div class="mx-auto max-w-7xl space-y-6">
  <!-- Intro & Single File Dropzone -->
  <section class="mx-auto max-w-4xl space-y-5">
    <div class="text-center">
      <h2 class="text-2xl font-bold tracking-tight text-main sm:text-3xl">{i18n.t('converter.title')}</h2>
      <p class="mt-1.5 text-sm text-sub">{i18n.t('converter.subtitle')}</p>
    </div>

    {#if !selectedFile}
      <div
        role="button"
        tabindex="0"
        onclick={() => fileInputEl?.click()}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            fileInputEl?.click();
          }
        }}
        ondragover={(e) => {
          e.preventDefault();
          dragOver = true;
        }}
        ondragleave={(e) => {
          e.preventDefault();
          dragOver = false;
        }}
        ondrop={handleDrop}
        class="surface-card cursor-pointer border-2 border-dashed p-8 text-center transition-all sm:p-12 {dragOver
          ? 'border-indigo-500 bg-indigo-500/10'
          : 'hover:border-indigo-500/70'}"
      >
        <div class="mx-auto flex h-14 w-14 items-center justify-center rounded-2xl bg-indigo-500/15 text-indigo-400">
          <svg class="h-7 w-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
        </div>
        <p class="mt-4 text-base font-semibold text-main">{i18n.t('converter.dropTitle')}</p>
        <p class="mt-1 text-sm text-sub">
          {i18n.t('converter.dropSubtitle')} · {i18n.t('converter.maxSize')}: {config.maxUploadSizeMb} MB
        </p>

        <div class="mt-5 flex flex-wrap justify-center gap-1.5">
          {#each badgeGroups as badge}
            <span class="surface-elevated rounded-md px-2 py-0.5 text-[11px] font-medium text-sub">{badge}</span>
          {/each}
        </div>
      </div>
    {:else}
      <!-- Selected Single File Card -->
      {@const cat = getFileCategory(selectedFile.name)}
      {@const ext = (selectedFile.name.split('.').pop() || '').toUpperCase()}
      <div class="surface-card p-5">
        <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div class="flex min-w-0 items-center gap-3.5">
            <div class="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl text-xs font-bold {categoryBadgeColor(cat)}">
              {ext}
            </div>
            <div class="min-w-0">
              <p class="truncate text-sm font-semibold text-main">{selectedFile.name}</p>
              <p class="text-xs text-dim">{formatFileSize(selectedFile.size)}</p>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <button
              type="button"
              disabled={busy}
              onclick={() => fileInputEl?.click()}
              class="btn-secondary px-3 py-1.5 text-xs"
            >
              {i18n.t('converter.changeFile')}
            </button>
            <button
              type="button"
              disabled={busy}
              onclick={clearFile}
              class="btn-secondary px-3 py-1.5 text-xs text-red-400 hover:border-red-400"
            >
              {i18n.t('converter.removeFile')}
            </button>
          </div>
        </div>

        <!-- Model selector + Start button -->
        <div class="mt-4 flex flex-col gap-3 border-t border-[var(--border-subtle)] pt-4 sm:flex-row sm:items-end sm:justify-between">
          <div class="w-full sm:max-w-xs">
            <label for="conv-model" class="mb-1 block text-xs font-medium text-sub">
              {i18n.t('converter.modelLabel')}
            </label>
            <select
              id="conv-model"
              bind:value={selectedModel}
              disabled={busy || config.convertModels.length === 0}
              class="surface-input w-full rounded-lg px-3 py-2 text-sm"
            >
              {#if config.convertModels.length === 0}
                <option value="">{i18n.t('models.noneConfigured')}</option>
              {:else}
                {#each config.convertModels as model}
                  <option value={model.id}>{model.label}</option>
                {/each}
              {/if}
            </select>
          </div>

          <button
            type="button"
            disabled={busy}
            onclick={startConvert}
            class="btn-primary flex items-center justify-center gap-2 px-6 py-2.5 text-sm"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <span>{busy ? i18n.t('converter.converting') : i18n.t('converter.start')}</span>
          </button>
        </div>
      </div>
    {/if}

    <input bind:this={fileInputEl} type="file" accept={acceptAttr} onchange={handleFileInput} class="hidden" />

    <!-- Active Progress Card -->
    {#if busy}
      <div class="surface-card p-5">
        <div class="flex items-center justify-between gap-4">
          <div class="flex items-center gap-3.5">
            <div class="h-7 w-7 shrink-0 animate-spin rounded-full border-2 border-indigo-500 border-t-transparent"></div>
            <div>
              <p class="text-sm font-semibold text-main">
                {stage === 'uploading' ? i18n.t('converter.uploading') : i18n.t(jobMessageKey)}
              </p>
              {#if stage === 'uploading' && uploadProgress}
                <p class="mt-0.5 text-xs text-sub">
                  {i18n.t('converter.uploaded')}: {formatFileSize(uploadProgress.loaded)} / {formatFileSize(uploadProgress.total)}
                  · {i18n.t('converter.speed')}: {formatFileSize(uploadProgress.averageSpeed)}/s
                  · {i18n.t('converter.eta')}: {formatDuration(uploadProgress.etaSeconds, i18n.lang)}
                </p>
              {:else if mediaProgress}
                <p class="mt-0.5 text-xs text-sub">
                  FFmpeg {mediaProgress.percent}% · {i18n.t('converter.bitrate')}: {mediaProgress.bitrateKbps} kbps
                  · {i18n.t('converter.attempt')}: {mediaProgress.attempt}
                  · {i18n.t('converter.duration')}: {formatDuration(mediaProgress.durationSeconds, i18n.lang)}
                  · {i18n.t('converter.eta')}: {formatDuration(mediaProgress.etaSeconds, i18n.lang)}
                </p>
              {:else if aiStats}
                <p class="mt-0.5 text-xs text-sub">
                  {i18n.t('converter.aiLive')} · {aiStats.chars.toLocaleString()} {i18n.t('converter.chars')}
                  · {aiStats.words.toLocaleString()} {i18n.t('converter.words')}
                  · {i18n.t('converter.elapsed')}: {formatDuration(aiStats.elapsedSeconds, i18n.lang)}
                </p>
              {/if}
            </div>
          </div>

          <button
            type="button"
            disabled={cancelling}
            onclick={handleCancel}
            class="shrink-0 rounded-lg bg-red-600 px-3.5 py-2 text-xs font-semibold text-white transition-colors hover:bg-red-700 disabled:opacity-50"
          >
            {cancelling ? i18n.t('converter.cancelling') : i18n.t('converter.cancel')}
          </button>
        </div>

        <div class="mt-4 h-2 overflow-hidden rounded-full bg-slate-700/40">
          {#if stage === 'uploading' && uploadProgress}
            <div
              class="h-full rounded-full bg-gradient-to-r from-indigo-500 to-purple-500 transition-all duration-150"
              style="width: {uploadProgress.percent}%"
            ></div>
          {:else if aiStats}
            <div class="h-full w-full animate-pulse rounded-full bg-gradient-to-r from-indigo-500 via-purple-500 to-indigo-500"></div>
          {:else}
            <div
              class="h-full rounded-full bg-gradient-to-r from-indigo-500 to-purple-500 transition-all duration-300"
              style="width: {jobProgress}%"
            ></div>
          {/if}
        </div>
      </div>
    {/if}
  </section>

  <!-- Result Editor & Preview -->
  {#if hasResult}
    <section class="space-y-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <!-- View Tabs -->
        <div class="surface-elevated flex items-center gap-1 rounded-xl p-1">
          {#each ['markdown', 'preview', 'split'] as mode}
            <button
              type="button"
              onclick={() => (viewMode = mode as 'markdown' | 'preview' | 'split')}
              class="rounded-lg px-3 py-1.5 text-xs font-medium transition-all {viewMode === mode
                ? 'bg-indigo-600 text-white'
                : 'text-sub hover:text-main'}"
            >
              {i18n.t(`tabs.${mode}`)}
            </button>
          {/each}
        </div>

        <!-- Action Buttons -->
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
            class="btn-primary px-3.5 py-2 text-xs"
          >
            {exportingPdf ? i18n.t('toast.generatingPdf') : i18n.t('actions.downloadPdf')}
          </button>
          <button
            type="button"
            onclick={handleSendToSummarizer}
            class="rounded-lg bg-emerald-600 px-3.5 py-2 text-xs font-semibold text-white transition-colors hover:bg-emerald-700"
          >
            {i18n.t('converter.summarizeAction')}
          </button>
        </div>
      </div>

      <div class="grid gap-4 {viewMode === 'split' ? 'grid-cols-1 lg:grid-cols-2' : 'grid-cols-1'}">
        {#if viewMode === 'markdown' || viewMode === 'split'}
          <div class="surface-card overflow-hidden">
            <div class="flex items-center justify-between border-b border-[var(--border-subtle)] px-4 py-3">
              <span class="text-xs font-semibold text-sub">{i18n.t('tabs.markdown')}</span>
              <span class="text-xs text-dim">
                {charCount.toLocaleString()} {i18n.t('converter.chars')} · {wordCount.toLocaleString()} {i18n.t('converter.words')}
              </span>
            </div>
            <textarea
              bind:value={markdown}
              placeholder={i18n.t('converter.editorPlaceholder')}
              class="h-[520px] w-full resize-none bg-transparent p-4 font-mono text-sm leading-relaxed text-main focus:outline-none"
            ></textarea>
          </div>
        {/if}

        {#if viewMode === 'preview' || viewMode === 'split'}
          <div class="surface-card overflow-hidden">
            <div class="border-b border-[var(--border-subtle)] px-4 py-3">
              <span class="text-xs font-semibold text-sub">{i18n.t('tabs.preview')}</span>
            </div>
            <MarkdownPreview {markdown} placeholder={i18n.t('converter.previewEmpty')} />
          </div>
        {/if}
      </div>
    </section>
  {/if}
</div>
