import { api, type Comfort } from '../api';
import { h, clear, cls, adminPage } from '../ui';

function field(label: string, input: HTMLElement): HTMLElement {
  return h(
    'label',
    { class: 'block' },
    h('span', { class: 'block text-sm text-slate-300 mb-1' }, label),
    input,
  );
}

export async function renderSettings(app: HTMLElement): Promise<void> {
  clear(app);
  let c: Comfort;
  try {
    c = await api.getComfort();
  } catch (e) {
    app.append(adminPage('comfort', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }
  let targets: string[] = [];
  try {
    targets = (await api.backupTargets()).targets;
  } catch {
    /* ignore */
  }

  // ---- Slideshow ----
  const ssEnabled = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-brand' }) as HTMLInputElement;
  ssEnabled.checked = c.slideshow_enabled;
  const ssIdle = h('input', { type: 'number', min: 0, max: 3600, value: c.slideshow_idle, class: cls.input }) as HTMLInputElement;
  const ssInterval = h('input', { type: 'number', min: 1, max: 60, value: c.slideshow_interval, class: cls.input }) as HTMLInputElement;

  const comfortMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const saveComfort = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          await api.setComfort({
            language: c.language, // language is edited under „Allgemein"
            slideshow_enabled: ssEnabled.checked,
            slideshow_idle: Number(ssIdle.value) || 0,
            slideshow_interval: Number(ssInterval.value) || 5,
          });
          comfortMsg.textContent = 'Gespeichert ✓';
          setTimeout(() => (comfortMsg.textContent = ''), 2000);
        } catch (e) {
          comfortMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'Speichern',
  );

  // ---- Backup / export ----
  const pathInput = h('input', {
    type: 'text',
    value: targets[0] ?? '',
    placeholder: 'Zielverzeichnis (z. B. USB-Laufwerk)',
    class: cls.input,
  }) as HTMLInputElement;
  const targetSel = h(
    'select',
    { class: cls.input },
    h('option', { value: '' }, targets.length ? 'Laufwerk wählen …' : 'Keine Laufwerke erkannt'),
    ...targets.map((p) => h('option', { value: p }, p)),
  ) as HTMLSelectElement;
  targetSel.addEventListener('change', () => {
    if (targetSel.value) pathInput.value = targetSel.value;
  });

  const backupMsg = h('span', { class: 'text-sm min-h-[1.25rem]' });
  const backupBtn = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        backupMsg.className = 'text-sm min-h-[1.25rem] text-slate-300';
        backupMsg.textContent = 'Exportiere …';
        try {
          const r = await api.backup(pathInput.value);
          backupMsg.className = 'text-sm min-h-[1.25rem] text-emerald-400';
          backupMsg.textContent = `${r.copied} von ${r.total} Fotos exportiert nach ${r.path}`;
        } catch (e) {
          backupMsg.className = 'text-sm min-h-[1.25rem] text-red-400';
          backupMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'Jetzt sichern',
  );

  app.append(
    adminPage(
      'comfort',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Diashow & Sicherung'),
      h(
        'div',
        { class: cls.card + ' mb-6 space-y-3' },
        h('h2', { class: 'font-semibold' }, 'Diashow / Bildschirmschoner'),
        h('label', { class: 'flex items-center gap-3' }, ssEnabled, 'Diashow / Bildschirmschoner aktiv'),
        field('Start nach Leerlauf (Sekunden, 0 = aus)', ssIdle),
        field('Sekunden pro Bild', ssInterval),
        h('div', { class: 'flex items-center gap-3' }, saveComfort, comfortMsg),
      ),
      h(
        'div',
        { class: cls.card + ' space-y-3' },
        h('h2', { class: 'font-semibold' }, 'Sicherung / Export'),
        h('p', { class: 'text-sm text-slate-400' }, 'Kopiert alle Fotos in „snapstation-export" im Zielverzeichnis.'),
        field('Erkannte Laufwerke', targetSel),
        field('Zielverzeichnis', pathInput),
        h('div', { class: 'flex items-center gap-3' }, backupBtn, backupMsg),
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
