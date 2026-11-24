class TransferStore {
  pendingSummaryMarkdown = $state<string | null>(null);

  sendToSummarizer(markdown: string) {
    this.pendingSummaryMarkdown = markdown;
  }

  consumeForSummarizer(): string | null {
    const value = this.pendingSummaryMarkdown;
    this.pendingSummaryMarkdown = null;
    return value;
  }
}

export const transfer = new TransferStore();
