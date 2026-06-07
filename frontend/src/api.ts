// Typed client for the SnapStation backend.

export interface CameraState {
  state: 'disconnected' | 'idle' | 'previewing' | 'capturing' | 'error';
  message?: string;
}

export interface Status {
  version: string;
  setup_complete: boolean;
  event_name: string;
  camera: {
    info: { model: string; port: string; backend: string };
    state: CameraState;
  };
  photo_count: number;
  printers: unknown[];
}

export interface Photo {
  id: number;
  ulid: string;
  token: string;
  original_path: string;
  thumb_path: string | null;
  width: number | null;
  height: number | null;
  kind: string;
  created_at: string;
}

export interface SetupStatus {
  complete: boolean;
  event_name: string;
  language: string;
}

export interface ChromaKey {
  enabled: boolean;
  key_color: [number, number, number];
  tolerance: number;
  softness: number;
  despill: number;
}

export interface Calibration {
  key_color: [number, number, number];
  tolerance: number;
  softness: number;
  evenness: number;
  message: string;
}

export interface BackgroundItem {
  ulid: string;
  name: string;
  url: string;
  selected: boolean;
}

export interface Region {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface PrinterInfo {
  name: string;
  is_default: boolean;
  state: 'idle' | 'printing' | 'stopped' | 'unknown';
  message: string;
}

export interface PrintSettings {
  printer: string;
  copies: number;
  media: string;
  auto_print: boolean;
  limit: number;
}

export interface PrintJobView {
  id: number;
  printer: string;
  job_ref: string | null;
  status: string;
  message: string | null;
  created_at: string;
  token: string | null;
}

export interface Theme {
  primary: string;
  logo_url: string;
  title: string;
  subtitle: string;
  kiosk_bg_url: string;
}

export interface EmailSettings {
  enabled: boolean;
  host: string;
  port: number;
  user: string;
  from: string;
  has_password: boolean;
}

export interface Comfort {
  language: string;
  slideshow_enabled: boolean;
  slideshow_idle: number;
  slideshow_interval: number;
}

export interface CameraSettings {
  countdown_seconds: number;
  countdown_sound: boolean;
  mirror_preview: boolean;
  camera_index: number;
  /** 'live' | 'on_demand' | 'off' */
  preview_mode: string;
}

export interface CameraInfo {
  info: { model: string; port: string; backend: string };
  state: { state: string; message?: string };
  settings: CameraSettings;
  available: { index: number; name: string }[];
}

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init);
  if (!res.ok) {
    let message = res.statusText;
    try {
      const body = (await res.json()) as { error?: string };
      if (body.error) message = body.error;
    } catch {
      /* non-JSON error body */
    }
    throw new Error(message);
  }
  return (await res.json()) as T;
}

export const api = {
  status: () => request<Status>('/api/status'),
  setupStatus: () => request<SetupStatus>('/api/setup'),
  submitSetup: (body: { admin_password: string; event_name?: string; language?: string }) =>
    request<{ ok: boolean }>('/api/setup', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }),
  // Auth
  authStatus: () => request<{ authenticated: boolean }>('/api/auth'),
  login: (password: string) =>
    request<{ ok: boolean }>('/api/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password }),
    }),
  logout: () => request<{ ok: boolean }>('/api/logout', { method: 'POST' }),

  // Software update (GitHub)
  updateCheck: () =>
    request<{
      current: string;
      latest: string;
      update_available: boolean;
      release_url: string;
      can_self_update: boolean;
    }>('/api/update/check'),
  updateApply: () =>
    request<{ ok: boolean; updated_to: string; restart_required: boolean }>('/api/update/apply', {
      method: 'POST',
    }),

  // Camera / capture settings
  getCamera: () => request<CameraInfo>('/api/camera'),
  setCamera: (s: CameraSettings) =>
    request<{ ok: boolean }>('/api/camera', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(s),
    }),
  detectCamera: () =>
    request<{ info: { model: string; port: string; backend: string } }>('/api/camera/detect', {
      method: 'POST',
    }),

  capture: () => request<Photo>('/api/capture', { method: 'POST' }),
  photos: (limit = 60, offset = 0) =>
    request<Photo[]>(`/api/photos?limit=${limit}&offset=${offset}`),
  photoUrl: (token: string) => `/p/${token}`,
  thumbUrl: (token: string) => `/p/${token}/thumb`,
  previewStream: () => '/api/preview/stream',

  // Chroma key
  getChroma: () => request<ChromaKey>('/api/chroma'),
  putChroma: (key: ChromaKey) =>
    request<{ ok: boolean }>('/api/chroma', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(key),
    }),
  calibrate: (region: Region) =>
    request<Calibration>('/api/chroma/calibrate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(region),
    }),
  /** POSTs the config and returns an object URL for the rendered JPEG (caller revokes it). */
  chromaPreviewBlob: async (key: ChromaKey, view: 'composite' | 'matte'): Promise<string> => {
    const res = await fetch('/api/chroma/preview', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key, view }),
    });
    if (!res.ok) throw new Error('Vorschau fehlgeschlagen');
    return URL.createObjectURL(await res.blob());
  },

  // Backgrounds
  listBackgrounds: () =>
    request<{ backgrounds: BackgroundItem[]; selected: string }>('/api/backgrounds'),
  uploadBackground: (name: string, bytes: Blob) =>
    request<{ ulid: string }>(`/api/backgrounds?name=${encodeURIComponent(name)}`, {
      method: 'POST',
      headers: { 'Content-Type': bytes.type || 'application/octet-stream' },
      body: bytes,
    }),
  selectBackground: (ulid: string) =>
    request<{ ok: boolean }>('/api/chroma/background', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ulid }),
    }),
  deleteBackground: (ulid: string) =>
    request<{ ok: boolean }>(`/api/backgrounds/${ulid}`, { method: 'DELETE' }),

  // Printing
  printers: () => request<{ printers: PrinterInfo[]; settings: PrintSettings }>('/api/printers'),
  print: (token: string) =>
    request<{ ok: boolean; job_id: number; printer: string; status: string }>('/api/print', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token }),
    }),
  printJobs: () => request<{ jobs: PrintJobView[] }>('/api/print/jobs'),
  getPrintSettings: () => request<PrintSettings>('/api/print/settings'),
  setPrintSettings: (s: PrintSettings) =>
    request<{ ok: boolean }>('/api/print/settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(s),
    }),

  // Sharing
  qrUrl: (token: string) => `/api/qr?data=${encodeURIComponent(location.origin + '/d/' + token)}`,
  downloadPageUrl: (token: string) => `${location.origin}/d/${token}`,
  shareEmail: (token: string, to: string) =>
    request<{ ok: boolean }>('/api/share/email', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token, to }),
    }),
  getEmailSettings: () => request<EmailSettings>('/api/email/settings'),
  setEmailSettings: (s: Omit<EmailSettings, 'has_password'> & { password?: string }) =>
    request<{ ok: boolean }>('/api/email/settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(s),
    }),

  // Branding
  getTheme: () => request<Theme>('/api/theme'),
  putTheme: (t: Theme) =>
    request<{ ok: boolean }>('/api/theme', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(t),
    }),
  uploadLogo: (bytes: Blob) =>
    request<{ ok: boolean; logo_url: string }>('/api/branding/logo', {
      method: 'POST',
      headers: { 'Content-Type': bytes.type || 'application/octet-stream' },
      body: bytes,
    }),
  uploadKioskBg: (bytes: Blob) =>
    request<{ ok: boolean; kiosk_bg_url: string }>('/api/branding/kiosk-bg', {
      method: 'POST',
      headers: { 'Content-Type': bytes.type || 'application/octet-stream' },
      body: bytes,
    }),

  // Comfort
  getComfort: () => request<Comfort>('/api/comfort'),
  setComfort: (c: Comfort) =>
    request<{ ok: boolean }>('/api/comfort', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(c),
    }),
  backupTargets: () => request<{ targets: string[] }>('/api/backup/targets'),
  backup: (path: string) =>
    request<{ copied: number; total: number; path: string }>('/api/backup', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    }),
};
