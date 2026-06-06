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

export async function renderCamera(app: HTMLElement): Promise<void> {
  clear(app);
  let cam: CameraInfo;
  try {
    cam = await api.getCamera();
  } catch (e) {
    app.append(adminPage('camera', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }

  const stateColor =
    cam.state.state === 'error'
      ? 'text-red-400'
      : cam.state.state === 'disconnected'
        ? 'text-amber-400'
        : 'text-emerald-400';

  // --- Status card ---
  const statusMsg = h('span', { class: 'text-sm min-h-[1.25rem]' });
  const detectBtn = h(
    'button',
    {
      class: cls.ghost,
      onclick: async () => {
        statusMsg.className = 'text-sm min-h-[1.25rem] text-slate-300';
        statusMsg.textContent = 'Suche Kamera …';
        try {
          const r = await api.detectCamera();
          statusMsg.className = 'text-sm min-h-[1.25rem] text-emerald-400';
          statusMsg.textContent = `Erkannt: ${r.info.model}`;
        } catch (e) {
          statusMsg.className = 'text-sm min-h-[1.25rem] text-red-400';
          statusMsg.textContent = e instanceof Error ? e.message : 'Keine Kamera gefunden';
        }
      },
    },
    'Kamera neu erkennen',
  );

  const statusCard = h(
    'div',
    { class: cls.card + ' mb-6 space-y-2' },
    h('h2', { class: 'font-semibold mb-1' }, 'Status'),
    row('Modell', cam.info.model),
    row('Backend', cam.info.backend),
    row('Verbindung', h('span', { class: stateColor }, cam.state.state)),
    h('div', { class: 'flex items-center gap-3 pt-2' }, detectBtn, statusMsg),
  );

  // --- Capture settings ---
  const countdown = h('input', {
    type: 'number',
    min: 0,
    max: 10,
    value: cam.settings.countdown_seconds,
    class: cls.input,
  }) as HTMLInputElement;
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
          await api.setCamera({
            countdown_seconds: Number(countdown.value) || 0,
            countdown_sound: sound.checked,
            mirror_preview: mirror.checked,
          });
          saveMsg.textContent = 'Gespeichert ✓';
          setTimeout(() => (saveMsg.textContent = ''), 2000);
        } catch (e) {
          saveMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'Speichern',
  );

  const settingsCard = h(
    'div',
    { class: cls.card + ' space-y-3' },
    h('h2', { class: 'font-semibold' }, 'Aufnahme'),
    field('Countdown-Dauer (Sekunden, 0 = aus)', countdown),
    h('label', { class: 'flex items-center gap-3' }, sound, 'Countdown-Ton'),
    h('label', { class: 'flex items-center gap-3' }, mirror, 'Vorschau spiegeln (Selfie-Ansicht)'),
    h('div', { class: 'flex items-center gap-3 pt-1' }, saveBtn, saveMsg),
  );

  app.append(
    adminPage(
      'camera',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Kamera'),
      statusCard,
      settingsCard,
      h(
        'p',
        { class: 'mt-6 text-xs text-slate-500' },
        'Kamera-Hardwareparameter (ISO, Blende, Belichtung) werden auf der Ubuntu-Station ' +
          'mit angeschlossener DSLR über gphoto2 hier ergänzt.',
      ),
    ),
  );
}

function row(label: string, value: string | Node): HTMLElement {
  return h(
    'div',
    { class: 'flex justify-between gap-4 text-sm' },
    h('span', { class: 'text-slate-400' }, label),
    typeof value === 'string' ? h('span', { class: 'font-medium' }, value) : value,
  );
}
