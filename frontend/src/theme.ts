import { api, type Theme } from './api';

let current: Theme = {
  primary: '#0ea5e9',
  background: '#0b0f17',
  logo_url: '',
  title: '',
  subtitle: '',
  kiosk_bg_url: '',
};

/** The most recently applied theme (for views that show the logo/title). */
export function theme(): Theme {
  return current;
}

/** Set the brand colour live without a round-trip (for the design preview). */
export function setBrandColor(hex: string): void {
  document.documentElement.style.setProperty('--color-brand', hex || '#0ea5e9');
}

/** Set the app background colour live (for the design preview). */
export function setBackgroundColor(hex: string): void {
  document.documentElement.style.setProperty('--color-bg', hex || '#0b0f17');
}

/** Fetch the saved theme and apply it (brand + background colours via CSS variables). */
export async function applyTheme(): Promise<void> {
  try {
    current = await api.getTheme();
    setBrandColor(current.primary);
    setBackgroundColor(current.background);
  } catch {
    /* keep defaults if backend unreachable */
  }
}
