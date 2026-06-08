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

  // ---- Status / detect ----
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

  // ---- Device selection ----
  const camSel = h('select', { class: cls.input }) as HTMLSelectElement;
  if (cam.available.length) {
    for (const c of cam.available) camSel.append(h('option', { value: String(c.index) }, `${c.index}: ${c.name}`));
  } else {
    camSel.append(h('option', { value: '0' }, 'Standardkamera'));
  }
  camSel.value = String(cam.settings.camera_index);

  const saveMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const saveBtn = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          // Preserve capture settings; only change the selected device here.
          await api.setCamera({ ...cam.settings, camera_index: Number(camSel.value) || 0 });
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
        { class: cls.card + ' space-y-3' },
        h('h2', { class: 'font-semibold' }, 'Gerät'),
        field('Kamera', camSel, cam.available.length > 1 ? 'Mehrere Kameras erkannt — wähle die gewünschte.' : undefined),
        h('div', { class: 'flex items-center gap-3 pt-1' }, saveBtn, saveMsg),
      ),
      h(
        'p',
        { class: 'mt-4 text-xs text-slate-500' },
        'Countdown, Vorschau & Spiegeln stellst du unter „Aufnahme" ein. DSLR-Hardwareparameter (ISO/Blende) folgen auf der Ubuntu-Station.',
      ),
    ),
  );
}
