import { api, type CameraInfo, type CollageSettings } from '../api';
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

function row(label: string, value: string | Node): HTMLElement {
  return h(
    'div',
    { class: 'flex justify-between gap-4 text-sm' },
    h('span', { class: 'text-slate-400' }, label),
    typeof value === 'string' ? h('span', { class: 'font-medium' }, value) : value,
  );
}

export async function renderCamera(app: HTMLElement): Promise<void> {
  clear(app);
  let cam: CameraInfo;
  let collage: CollageSettings;
  try {
    [cam, collage] = await Promise.all([api.getCamera(), api.getCollage()]);
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

  // ---- Status ----
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
          statusMsg.textContent = e instanceof Error ? e.message : 'Keine Kamera';
        }
      },
    },
    'Kamera neu erkennen',
  );

  // ---- Selection + capture ----
  const camSel = h('select', { class: cls.input }) as HTMLSelectElement;
  if (cam.available.length) {
    for (const c of cam.available) camSel.append(h('option', { value: String(c.index) }, `${c.index}: ${c.name}`));
  } else {
    camSel.append(h('option', { value: '0' }, 'Standardkamera'));
  }
  camSel.value = String(cam.settings.camera_index);

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
          await api.setCamera({
            countdown_seconds: Number(countdown.value) || 0,
            countdown_sound: sound.checked,
            mirror_preview: mirror.checked,
            camera_index: Number(camSel.value) || 0,
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

  // ---- Collage / multi-shot ----
  const colEnabled = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-brand' }) as HTMLInputElement;
  colEnabled.checked = collage.enabled;
  const colLayout = h(
    'select',
    { class: cls.input },
    h('option', { value: 'grid2x2' }, '2×2 Raster (4 Bilder)'),
    h('option', { value: 'duo1x2' }, '2 Bilder untereinander'),
    h('option', { value: 'strip1x3' }, '3er-Streifen (3 Bilder)'),
    h('option', { value: 'strip1x4' }, '4er-Streifen (4 Bilder)'),
    h('option', { value: 'grid2x3' }, '2×3 Raster (6 Bilder)'),
  ) as HTMLSelectElement;
  colLayout.value = collage.layout || 'grid2x2';
  const colCountdown = h('input', { type: 'number', min: 0, max: 10, value: collage.countdown_seconds, class: cls.input }) as HTMLInputElement;
  const colReview = h('input', { type: 'number', min: 0, max: 10, value: collage.review_seconds, class: cls.input }) as HTMLInputElement;
  const colMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const colSave = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          await api.setCollage({
            enabled: colEnabled.checked,
            layout: colLayout.value,
            countdown_seconds: Number(colCountdown.value) || 0,
            review_seconds: Number(colReview.value) || 0,
          });
          colMsg.className = 'text-sm text-emerald-400 min-h-[1.25rem]';
          colMsg.textContent = 'Gespeichert ✓';
          setTimeout(() => (colMsg.textContent = ''), 2000);
        } catch (e) {
          colMsg.className = 'text-sm text-red-400 min-h-[1.25rem]';
          colMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'Speichern',
  );

  app.append(
    adminPage(
      'camera',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Kamera'),
      h(
        'div',
        { class: cls.card + ' mb-6 space-y-2' },
        h('h2', { class: 'font-semibold mb-1' }, 'Status'),
        row('Modell', cam.info.model),
        row('Backend', cam.info.backend),
        row('Verbindung', h('span', { class: stateColor }, cam.state.state)),
        h('div', { class: 'flex items-center gap-3 pt-2' }, detectBtn, statusMsg),
      ),
      h(
        'div',
        { class: cls.card + ' mb-6 space-y-3' },
        h('h2', { class: 'font-semibold' }, 'Aufnahme & Vorschau'),
        field('Kamera', camSel, cam.available.length > 1 ? 'Mehrere Kameras erkannt — wähle die gewünschte.' : undefined),
        field('Live-Vorschau', modeSel, '„Erst beim Auslösen" und „Aus" zeigen das Hintergrundbild unten.'),
        field('Countdown-Dauer (Sekunden, 0 = aus)', countdown),
        h('label', { class: 'flex items-center gap-3' }, sound, 'Countdown-Ton'),
        h('label', { class: 'flex items-center gap-3' }, mirror, 'Vorschau spiegeln (Selfie-Ansicht)'),
        h('div', { class: 'flex items-center gap-3 pt-1' }, saveBtn, saveMsg),
      ),
      h(
        'div',
        { class: cls.card + ' mb-6 space-y-3' },
        h('h2', { class: 'font-semibold' }, 'Collage / Mehrfach-Aufnahme'),
        h('label', { class: 'flex items-center gap-3' }, colEnabled, 'Collage-Aufnahme erlauben'),
        field('Layout', colLayout, 'Bestimmt Anordnung und Anzahl der Aufnahmen.'),
        field('Countdown vor jedem Bild (Sekunden)', colCountdown),
        field('Anzeige je Bild zwischen den Aufnahmen (Sekunden)', colReview),
        h('div', { class: 'flex items-center gap-3 pt-1' }, colSave, colMsg),
      ),
      h(
        'p',
        { class: 'mt-2 text-xs text-slate-500' },
        'Das Hintergrundbild für die Vorschaumodi „aus" / „erst beim Auslösen" stellst du unter Design ein.',
      ),
      h(
        'p',
        { class: 'mt-6 text-xs text-slate-500' },
        'DSLR-Hardwareparameter (ISO/Blende/Belichtung über gphoto2) folgen auf der Ubuntu-Station.',
      ),
    ),
  );
}
