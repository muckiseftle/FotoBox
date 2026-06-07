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

  const result = h('input', { type: 'number', min: 0, max: 120, value: k.result_seconds, class: cls.input }) as HTMLInputElement;

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
            result_seconds: Number(result.value) || 0,
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

  app.append(
    adminPage(
      'kiosk',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Kiosk (Gäste-Ansicht)'),
      h(
        'div',
        { class: cls.card + ' space-y-4' },
        h('h2', { class: 'font-semibold' }, 'Bedienelemente'),
        gallery.row,
        print.row,
        qr.row,
        h(
          'label',
          { class: 'block pt-1' },
          h('span', { class: 'block text-sm text-slate-300 mb-1' }, 'Ergebnis automatisch ausblenden nach (Sekunden, 0 = manuell)'),
          result,
          h('span', { class: 'block text-xs text-slate-500 mt-1' }, 'Kehrt nach der eingestellten Zeit automatisch zur Live-Ansicht zurück — ideal für unbeaufsichtigten Betrieb.'),
        ),
        h('div', { class: 'flex items-center gap-3 pt-1' }, save, msg),
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
