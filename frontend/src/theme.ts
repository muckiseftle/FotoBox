import { api, type Theme } from './api';

let current: Theme = { primary: '#0ea5e9', logo_url: '', title: '', subtitle: '', kiosk_bg_url: '' };

/** The most recently applied theme (for views that show the logo/title). */
export function theme(): Theme {
  return current;
}

/** Set the brand colour live without a round-trip (for the design preview). */
export function setBrandColor(hex: string): void {
  document.documentElement.style.setProperty('--color-brand', hex || '#0ea5e9');
}

/** Fetch the saved theme and apply it (brand colour via CSS variable). */
export async function applyTheme(): Promise<void> {
  try {
    current = await api.getTheme();
    setBrandColor(current.primary);
  } catch {
    /* keep defaults if backend unreachable */
  }
}
