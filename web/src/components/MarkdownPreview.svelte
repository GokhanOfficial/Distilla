<script lang="ts">
  import { renderSafeMarkdown, highlightCodeBlocks } from '../lib/markdown';

  let {
    markdown = '',
    placeholder = '',
    containerEl = $bindable<HTMLDivElement | null>(null),
    onscroll,
    heightClass = 'h-[520px]',
  }: {
    markdown: string;
    placeholder?: string;
    containerEl?: HTMLDivElement | null;
    onscroll?: (e: Event) => void;
    heightClass?: string;
  } = $props();

  let renderedHtml = $derived(renderSafeMarkdown(markdown));

  $effect(() => {
    // Re-run syntax highlighting whenever renderedHtml updates
    void renderedHtml;
    if (containerEl) {
      highlightCodeBlocks(containerEl);
    }
  });
</script>

<div
  bind:this={containerEl}
  {onscroll}
  class="doc-preview {heightClass} overflow-auto p-6"
>
  {#if markdown.trim()}
    {@html renderedHtml}
  {:else if placeholder}
    <p class="mt-20 text-center text-sm italic text-dim">{placeholder}</p>
  {/if}
</div>
