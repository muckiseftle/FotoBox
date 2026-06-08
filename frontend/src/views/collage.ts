import { api, type CollageSettings } from '../api';
import { h, clear, cls, adminPage } from '../ui';

function field(label: string, input: HTMLElement, hint?: string): HTMLElement {
  return h(
    'label',
    { class: 'block' },
    h('span', { class: 'block text-sm text-slate-300 mb-1' }, label),
    input,
    hint ? h('span', { class: 'block text-xs text-slate-500 mt-1' }, hint) : '',
  );
}

export async function renderCollage(app: HTMLElement): Promise<void> {
  clear(app);
  let c: CollageSettings;
  try {
    c = await api.getCollage();
  } catch (e) {
    app.append(adminPage('collage', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }

  const enabled = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-brand' }) as HTMLInputElement;
  enabled.checked = c.enabled;
  const layout = h(
    'select',
    { class: cls.input },
    h('option', { value: 'grid2x2' }, '2×2 Raster (4 Bilder)'),
    h('option', { value: 'duo1x2' }, '2 Bilder untereinander'),
    h('option', { value: 'strip1x3' }, '3er-Streifen (3 Bilder)'),
    h('option', { value: 'strip1x4' }, '4er-Streifen (4 Bilder)'),
    h('option', { value: 'grid2x3' }, '2×3 Raster (6 Bilder)'),
  ) as HTMLSelectElement;
  layout.value = c.layout || 'grid2x2';
  const countdown = h('input', { type: 'number', min: 0, max: 10, value: c.countdown_seconds, class: cls.input }) as HTMLInputElement;
  const review = h('input', { type: 'number', min: 0, max: 10, value: c.review_seconds, class: cls.input }) as HTMLInputElement;

  const msg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const save = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          await api.setCollage({
            enabled: enabled.checked,
            layout: layout.value,
            countdown_seconds: Number(countdown.value) || 0,
            review_seconds: Number(review.value) || 0,
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
      'collage',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Collage / Mehrfach-Aufnahme'),
      h(
        'div',
        { class: cls.card + ' space-y-3' },
        h('p', { class: 'text-sm text-slate-400' }, 'Gäste nehmen mehrere Bilder nacheinander auf, die zu einer Collage zusammengesetzt werden. Im Kiosk erscheint dafür ein Collage-Button neben dem Auslöser.'),
        h('label', { class: 'flex items-center gap-3' }, enabled, 'Collage-Aufnahme erlauben'),
        field('Layout', layout, 'Bestimmt Anordnung und Anzahl der Aufnahmen.'),
        field('Countdown vor jedem Bild (Sekunden)', countdown),
        field('Anzeige je Bild zwischen den Aufnahmen (Sekunden)', review),
        h('div', { class: 'flex items-center gap-3 pt-1' }, save, msg),
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
