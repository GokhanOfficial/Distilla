export type ToastType = 'info' | 'success' | 'warning' | 'error';

export interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
}

class ToastStore {
  items = $state<ToastItem[]>([]);
  private nextId = 1;

  show(message: string, type: ToastType = 'info', durationMs = 3500) {
    const id = this.nextId++;
    this.items = [...this.items, { id, message, type }];
    setTimeout(() => {
      this.dismiss(id);
    }, durationMs);
  }

  dismiss(id: number) {
    this.items = this.items.filter((item) => item.id !== id);
  }
}

export const toasts = new ToastStore();
