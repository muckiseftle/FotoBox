import { api, type KioskSettings } from '../api';
import { h, clear, cls, adminPage } from '../ui';

function toggle(label: string, checked: boolean, hint?: string): { row: HTMLElement; input: HTMLInputElement } {
  const input = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-brand' }) as HTMLInputElement;
  input.checked = checked;
  const row = h(
    'div',
    { class: 'space-y-1' },
    h('label', { class: 'flex items-center gap-3' }, input, label),
    hint ? h('span', { class: 'block text-xs text-slate-500 ml-8' }, hint) : '',
  );
  return { row, input };
}

export async function renderKioskSettings(app: HTMLElement): Promise<void> {
  clear(app);
  let k: KioskSettings;
  try {
    k = await api.getKiosk();
  } catch (e) {
    app.append(adminPage('kiosk', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }

  const gallery = toggle('Galerie-Button anzeigen', k.show_gallery, 'Button neben dem Auslöser, der zur Foto-Galerie führt.');
  const print = toggle('Drucken-Button anzeigen', k.show_print, 'Erlaubt Gästen, das aufgenommene Foto direkt zu drucken.');
  const qr = toggle('QR-Code anzeigen', k.show_qr, 'Zeigt nach der Aufnahme einen QR-Code zum Download auf dem Handy.');
  const qrLogo = toggle('Logo im QR-Code', k.qr_logo, 'Bettet das Event-Logo (aus Design) in die Mitte des QR-Codes ein.');
  const delAfter = toggle('Löschen nach der Aufnahme erlauben', k.allow_delete_after_capture, 'Gäste können das gerade aufgenommene Bild direkt verwerfen.');
  const delGallery = toggle('Löschen in der Galerie erlauben', k.allow_delete_gallery, 'Gäste können Bilder in der Galerie löschen.');

  const result = h('input', { type: 'number', min: 0, max: 120, value: k.result_seconds, class: cls.input }) as HTMLInputElement;
  const galleryIdle = h('input', { type: 'number', min: 0, max: 600, value: k.gallery_idle_seconds, class: cls.input }) as HTMLInputElement;

  const msg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const save = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          await api.setKiosk({
            show_gallery: gallery.input.checked,
            show_print: print.input.checked,
            show_qr: qr.input.checked,
            qr_logo: qrLogo.input.checked,
            allow_delete_after_capture: delAfter.input.checked,
            allow_delete_gallery: delGallery.input.checked,
            result_seconds: Number(result.value) || 0,
            gallery_idle_seconds: Number(galleryIdle.value) || 0,
          });
          msg.className = 'text-sm text-emerald-400 min-h-[1.25rem]';
          msg.textContent = 'Gespeichert ✓';
          setTimeout(() => (msg.textContent = ''), 2000);
        } catch (e) {
          msg.className = 'text-sm text-red-400 min-h-[1.25rem]';
          msg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'Speichern',
  );

  const numField = (label: string, input: HTMLElement, hint: string) =>
    h(
      'label',
      { class: 'block pt-1' },
      h('span', { class: 'block text-sm text-slate-300 mb-1' }, label),
      input,
      h('span', { class: 'block text-xs text-slate-500 mt-1' }, hint),
    );

  app.append(
    adminPage(
      'kiosk',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Kiosk (Gäste-Ansicht)'),
      h(
        'div',
        { class: cls.card + ' mb-6 space-y-4' },
        h('h2', { class: 'font-semibold' }, 'Bedienelemente'),
        gallery.row,
        print.row,
        qr.row,
        qrLogo.row,
      ),
      h(
        'div',
        { class: cls.card + ' mb-6 space-y-4' },
        h('h2', { class: 'font-semibold' }, 'Löschen'),
        delAfter.row,
        delGallery.row,
      ),
      h(
        'div',
        { class: cls.card + ' space-y-2' },
        h('h2', { class: 'font-semibold' }, 'Automatik'),
        numField(
          'Ergebnis automatisch ausblenden nach (Sekunden, 0 = manuell)',
          result,
          'Kehrt nach dieser Zeit automatisch zur Live-Ansicht zurück.',
        ),
        numField(
          'Galerie automatisch verlassen nach Inaktivität (Sekunden, 0 = aus)',
          galleryIdle,
          'Kehrt aus der Galerie ohne Interaktion zurück zur Kiosk-Ansicht.',
        ),
        h('div', { class: 'flex items-center gap-3 pt-2' }, save, msg),
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
