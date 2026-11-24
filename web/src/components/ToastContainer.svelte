<script lang="ts">
  import { toasts, type ToastType } from '../lib/toast.svelte';

  function badgeStyle(type: ToastType): string {
    switch (type) {
      case 'success':
        return 'border-emerald-500/40 bg-emerald-950/90 text-emerald-100';
      case 'error':
        return 'border-red-500/40 bg-red-950/90 text-red-100';
      case 'warning':
        return 'border-amber-500/40 bg-amber-950/90 text-amber-100';
      default:
        return 'border-indigo-500/40 bg-slate-900/95 text-slate-100';
    }
  }
</script>

<div class="pointer-events-none fixed bottom-4 right-4 z-[70] flex max-w-sm flex-col gap-2">
  {#each toasts.items as item (item.id)}
    <div
      class="pointer-events-auto flex items-center gap-3 rounded-xl border px-4 py-3 text-sm shadow-xl backdrop-blur-md {badgeStyle(item.type)}"
      role="status"
    >
      <span class="flex-1 leading-snug">{item.message}</span>
      <button
        type="button"
        onclick={() => toasts.dismiss(item.id)}
        class="opacity-70 transition-opacity hover:opacity-100"
        aria-label="Close"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  {/each}
</div>
