import { api, type CameraInfo } from '../api';
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

export async function renderCapture(app: HTMLElement): Promise<void> {
  clear(app);
  let cam: CameraInfo;
  try {
    cam = await api.getCamera();
  } catch (e) {
    app.append(adminPage('capture', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }

  const modeSel = h(
    'select',
    { class: cls.input },
    h('option', { value: 'live' }, 'Immer live'),
    h('option', { value: 'on_demand' }, 'Erst beim Auslösen'),
    h('option', { value: 'off' }, 'Aus (Hintergrundbild zeigen)'),
  ) as HTMLSelectElement;
  modeSel.value = cam.settings.preview_mode || 'live';

  const countdown = h('input', { type: 'number', min: 0, max: 10, value: cam.settings.countdown_seconds, class: cls.input }) as HTMLInputElement;
  const sound = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-brand' }) as HTMLInputElement;
  sound.checked = cam.settings.countdown_sound;
  const mirror = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-brand' }) as HTMLInputElement;
  mirror.checked = cam.settings.mirror_preview;

  const saveMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const saveBtn = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          // Preserve the selected device; only change capture behaviour here.
          await api.setCamera({
            ...cam.settings,
            countdown_seconds: Number(countdown.value) || 0,
            countdown_sound: sound.checked,
            mirror_preview: mirror.checked,
            preview_mode: modeSel.value,
          });
          saveMsg.className = 'text-sm text-emerald-400 min-h-[1.25rem]';
          saveMsg.textContent = 'Gespeichert ✓';
          setTimeout(() => (saveMsg.textContent = ''), 2000);
        } catch (e) {
          saveMsg.className = 'text-sm text-red-400 min-h-[1.25rem]';
          saveMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'Speichern',
  );

  app.append(
    adminPage(
      'capture',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Aufnahme'),
      h(
        'div',
        { class: cls.card + ' space-y-3' },
        h('h2', { class: 'font-semibold' }, 'Auslösen & Vorschau'),
        field('Live-Vorschau', modeSel, '„Erst beim Auslösen" und „Aus" zeigen das Kiosk-Hintergrundbild (unter Design).'),
        field('Countdown-Dauer (Sekunden, 0 = aus)', countdown),
        h('label', { class: 'flex items-center gap-3' }, sound, 'Countdown-Ton'),
        h('label', { class: 'flex items-center gap-3' }, mirror, 'Vorschau spiegeln (Selfie-Ansicht)'),
        h('div', { class: 'flex items-center gap-3 pt-1' }, saveBtn, saveMsg),
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
