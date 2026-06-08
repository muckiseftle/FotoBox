import { api } from './api';

export type Lang = 'de' | 'en';

const dict: Record<Lang, Record<string, string>> = {
  de: {
    gallery: 'Galerie',
    admin: 'Admin',
    home: 'Start',
    shutter: 'Foto auslösen',
    retake: 'Nochmal',
    toGallery: 'Zur Galerie',
    print: 'Drucken',
    printSending: 'Sende an Drucker …',
    printing: 'Wird gedruckt …',
    printed: 'Gedruckt ✓',
    printError: 'Druckfehler',
    scanHint: 'Scan zum Herunterladen aufs Handy',
    captureFailed: 'Aufnahme fehlgeschlagen',
    unknownError: 'Unbekannter Fehler',
    photos: 'Fotos',
    noPhotos: 'Noch keine Fotos.',
    download: 'Herunterladen',
    delete: 'Löschen',
    confirmDelete: 'Dieses Foto löschen?',
  },
  en: {
    gallery: 'Gallery',
    admin: 'Admin',
    home: 'Home',
    shutter: 'Take photo',
    retake: 'Retake',
    toGallery: 'To gallery',
    print: 'Print',
    printSending: 'Sending to printer …',
    printing: 'Printing …',
    printed: 'Printed ✓',
    printError: 'Print error',
    scanHint: 'Scan to download to your phone',
    captureFailed: 'Capture failed',
    unknownError: 'Unknown error',
    photos: 'photos',
    noPhotos: 'No photos yet.',
    download: 'Download',
    delete: 'Delete',
    confirmDelete: 'Delete this photo?',
  },
};

let lang: Lang = 'de';

export function getLang(): Lang {
  return lang;
}

export function setLang(l: Lang): void {
  lang = l;
  try {
    localStorage.setItem('lang', l);
  } catch {
    /* private mode */
  }
}

export function t(key: string): string {
  return dict[lang][key] ?? dict.de[key] ?? key;
}

/** Resolve the initial language: local override, else the server default. */
export async function initLang(): Promise<void> {
  const stored = localStorage.getItem('lang');
  if (stored === 'de' || stored === 'en') {
    lang = stored;
    return;
  }
  try {
    const c = await api.getComfort();
    lang = c.language === 'en' ? 'en' : 'de';
  } catch {
    /* keep default */
  }
}
