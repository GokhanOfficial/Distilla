export interface ModelOption {
  id: string;
  label: string;
}

export interface FormatInfo {
  ext: string;
  name: string;
  category: string;
  mime: string;
}

export interface AppConfigResponse {
  appName: string;
  convertModels: ModelOption[];
  defaultConvertModel: string | null;
  summarizeModels: ModelOption[];
  defaultSummarizeModel: string | null;
  maxUploadSizeMb: number;
  mediaMaxOutputSizeMb: number;
  supportedExtensions: string[];
  formats: FormatInfo[];
  capabilities: {
    ai: boolean;
    ffmpeg: boolean;
    office: boolean;
    pdf: boolean;
  };
}

export interface UploadProgress {
  percent: number;
  loaded: number;
  total: number;
  averageSpeed: number;
  etaSeconds: number | null;
}

export interface MediaProgress {
  percent: number;
  bitrateKbps: number;
  attempt: number;
  durationSeconds: number;
  elapsedSeconds: number;
  etaSeconds: number | null;
}

export interface AiStats {
  chars: number;
  words: number;
  elapsedSeconds: number;
}

export interface JobSnapshot {
  jobId: string;
  kind: 'convert' | 'summarize';
  status: 'pending' | 'processing' | 'completed' | 'failed' | 'cancelled';
  phase: string;
  messageKey: string;
  filename?: string | null;
  generatedTitle?: string | null;
  markdown: string;
  progress: number;
  mediaProgress?: MediaProgress | null;
  aiStats?: AiStats | null;
  error?: string | null;
}

export interface JobStreamHandlers {
  onSnapshot?: (snap: JobSnapshot) => void;
  onState?: (state: {
    status: JobSnapshot['status'];
    phase: string;
    messageKey: string;
    progress: number;
    mediaProgress?: MediaProgress | null;
    aiStats?: AiStats | null;
  }) => void;
  onChunk?: (chunk: { delta: string; aiStats: AiStats }) => void;
  onDone?: (done: {
    status: JobSnapshot['status'];
    markdown: string;
    generatedTitle?: string | null;
    error?: string | null;
  }) => void;
  onError?: (err: string) => void;
}

export async function fetchAppConfig(): Promise<AppConfigResponse> {
  const res = await fetch('/api/config');
  if (!res.ok) throw new Error('Failed to load server configuration');
  return res.json();
}

export function startConversionUpload(
  file: File,
  model: string,
  onProgress: (p: UploadProgress) => void,
): { promise: Promise<{ jobId: string }>; abort: () => void } {
  const xhr = new XMLHttpRequest();
  const formData = new FormData();
  formData.append('model', model);
  formData.append('file', file);

  const startedAt = Date.now();

  const promise = new Promise<{ jobId: string }>((resolve, reject) => {
    xhr.upload.addEventListener('progress', (e) => {
      if (!e.lengthComputable) return;
      const elapsedSec = Math.max((Date.now() - startedAt) / 1000, 0.001);
      const averageSpeed = e.loaded / elapsedSec;
      const remaining = Math.max(e.total - e.loaded, 0);
      const etaSeconds = averageSpeed > 0 ? remaining / averageSpeed : null;
      const percent = Math.round((e.loaded / e.total) * 100);
      onProgress({
        percent,
        loaded: e.loaded,
        total: e.total,
        averageSpeed,
        etaSeconds,
      });
    });

    xhr.addEventListener('load', () => {
      try {
        const data = JSON.parse(xhr.responseText || '{}');
        if (xhr.status >= 200 && xhr.status < 300 && data.jobId) {
          resolve({ jobId: data.jobId });
        } else {
          reject(new Error(data.error || `Upload failed (HTTP ${xhr.status})`));
        }
      } catch {
        reject(new Error(`Upload failed (HTTP ${xhr.status})`));
      }
    });

    xhr.addEventListener('abort', () => {
      reject(new DOMException('Upload cancelled', 'AbortError'));
    });

    xhr.addEventListener('error', () => {
      reject(new Error('Network error during upload'));
    });

    xhr.open('POST', '/api/convert/jobs');
    xhr.send(formData);
  });

  return {
    promise,
    abort: () => xhr.abort(),
  };
}

export async function startSummarizeJob(markdown: string, model: string): Promise<{ jobId: string }> {
  const res = await fetch('/api/summarize/jobs', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ markdown, model }),
  });
  const data = await res.json();
  if (!res.ok || !data.jobId) {
    throw new Error(data.error || 'Failed to start summarization');
  }
  return { jobId: data.jobId };
}

export function subscribeToJob(
  kind: 'convert' | 'summarize',
  jobId: string,
  handlers: JobStreamHandlers,
): () => void {
  const url = `/api/${kind}/jobs/${encodeURIComponent(jobId)}/events`;
  const es = new EventSource(url);
  let finished = false;

  const close = () => {
    if (!finished) {
      finished = true;
      es.close();
    }
  };

  es.addEventListener('snapshot', (e) => {
    try {
      const snap: JobSnapshot = JSON.parse((e as MessageEvent).data);
      handlers.onSnapshot?.(snap);
      if (['completed', 'failed', 'cancelled'].includes(snap.status)) {
        handlers.onDone?.({
          status: snap.status,
          markdown: snap.markdown,
          generatedTitle: snap.generatedTitle,
          error: snap.error,
        });
        close();
      }
    } catch {
      // ignore malformed
    }
  });

  es.addEventListener('state', (e) => {
    try {
      handlers.onState?.(JSON.parse((e as MessageEvent).data));
    } catch {
      // ignore
    }
  });

  es.addEventListener('chunk', (e) => {
    try {
      handlers.onChunk?.(JSON.parse((e as MessageEvent).data));
    } catch {
      // ignore
    }
  });

  es.addEventListener('done', (e) => {
    try {
      handlers.onDone?.(JSON.parse((e as MessageEvent).data));
    } finally {
      close();
    }
  });

  es.onerror = () => {
    if (!finished) {
      close();
      handlers.onError?.('Connection to stream lost');
    }
  };

  return close;
}

export async function cancelJob(kind: 'convert' | 'summarize', jobId: string): Promise<void> {
  await fetch(`/api/${kind}/jobs/${encodeURIComponent(jobId)}/cancel`, {
    method: 'POST',
  });
}

export async function generateTitle(markdown: string, model?: string): Promise<string> {
  const res = await fetch('/api/generate-title', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ markdown, model }),
  });
  const data = await res.json();
  return data.title || 'document';
}

export async function generatePdfBlob(markdown: string, filename: string): Promise<{ blob: Blob; filename: string }> {
  const res = await fetch('/api/pdf', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ markdown, filename }),
  });

  if (!res.ok) {
    let errMsg = `PDF generation failed (HTTP ${res.status})`;
    try {
      const errData = await res.json();
      if (errData.error) errMsg = errData.error;
    } catch {
      // non-json error
    }
    throw new Error(errMsg);
  }

  const blob = await res.blob();
  const safeFilename = filename.endsWith('.pdf') ? filename : `${filename}.pdf`;
  return { blob, filename: safeFilename };
}
