import { api, type PrinterInfo, type PrintJobView, type PrintSettings } from '../api';
import { h, clear, cls, adminPage } from '../ui';

function stateColor(state: string): string {
  if (state === 'stopped' || state === 'error') return 'text-red-400';
  if (state === 'unknown') return 'text-amber-400';
  return 'text-emerald-400';
}

function printerCard(p: PrinterInfo): HTMLElement {
  return h(
    'div',
    { class: cls.card },
    h(
      'div',
      { class: 'flex items-center justify-between mb-1' },
      h('span', { class: 'font-medium truncate' }, p.name),
      p.is_default ? h('span', { class: 'text-xs text-sky-400' }, 'Standard') : '',
    ),
    h('div', { class: 'text-sm ' + stateColor(p.state) }, p.message),
  );
}

function jobRow(j: PrintJobView): HTMLElement {
  const badge = h(
    'span',
    {
      class:
        'text-xs px-2 py-0.5 rounded-full ' +
        (j.status === 'error'
          ? 'bg-red-500/20 text-red-300'
          : j.status === 'done'
            ? 'bg-emerald-500/20 text-emerald-300'
            : 'bg-sky-500/20 text-sky-300'),
    },
    j.status,
  );
  return h(
    'div',
    { class: 'flex items-center gap-3 py-2 border-b border-slate-800 last:border-0' },
    j.token
      ? h('img', { class: 'w-12 h-12 rounded object-cover bg-slate-800', src: api.thumbUrl(j.token), alt: '' })
      : h('div', { class: 'w-12 h-12 rounded bg-slate-800' }),
    h(
      'div',
      { class: 'min-w-0 flex-1' },
      h('div', { class: 'text-sm truncate' }, j.printer),
      h('div', { class: 'text-xs text-slate-400 truncate' }, j.message ?? j.created_at),
    ),
    badge,
  );
}

function field(label: string, input: HTMLElement): HTMLElement {
  return h(
    'label',
    { class: 'block mb-3' },
    h('span', { class: 'block text-sm text-slate-300 mb-1' }, label),
    input,
  );
}

export async function renderPrint(app: HTMLElement): Promise<void> {
  clear(app);
  let data: { printers: PrinterInfo[]; settings: PrintSettings };
  try {
    data = await api.printers();
  } catch (e) {
    app.append(adminPage('print', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }
  const s = data.settings;

  const printerSel = h(
    'select',
    { class: cls.input },
    h('option', { value: '' }, 'Standarddrucker'),
    ...data.printers.map((p) => h('option', { value: p.name }, p.name)),
  ) as HTMLSelectElement;
  printerSel.value = s.printer;
  const copies = h('input', { type: 'number', min: 1, max: 20, value: s.copies, class: cls.input }) as HTMLInputElement;
  const media = h('input', {
    type: 'text',
    value: s.media,
    placeholder: 'z. B. om_4x6_4x6in (leer = Standard)',
    class: cls.input,
  }) as HTMLInputElement;
  const limit = h('input', { type: 'number', min: 0, max: 50, value: s.limit, class: cls.input }) as HTMLInputElement;
  const autoPrint = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-sky-500' }) as HTMLInputElement;
  autoPrint.checked = s.auto_print;

  const saveMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const saveBtn = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          await api.setPrintSettings({
            printer: printerSel.value,
            copies: Number(copies.value) || 1,
            media: media.value,
            auto_print: autoPrint.checked,
            limit: Number(limit.value) || 0,
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

  const jobsBox = h('div', { class: 'divide-y divide-slate-800' });
  const loadJobs = async () => {
    try {
      const { jobs } = await api.printJobs();
      jobsBox.replaceChildren(
        ...(jobs.length ? jobs.map(jobRow) : [h('p', { class: 'text-slate-400' }, 'Noch keine Druckaufträge.')]),
      );
    } catch {
      /* ignore transient */
    }
  };
  await loadJobs();

  app.append(
    adminPage(
      'print',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Drucken'),
      h(
        'div',
        { class: 'grid gap-4 sm:grid-cols-2 lg:grid-cols-3 mb-6' },
        ...(data.printers.length
          ? data.printers.map(printerCard)
          : [h('p', { class: 'text-slate-400' }, 'Kein Drucker erkannt.')]),
      ),
      h(
        'div',
        { class: cls.card + ' mb-6' },
        h('h2', { class: 'font-semibold mb-3' }, 'Druck-Einstellungen'),
        field('Drucker', printerSel),
        field('Kopien', copies),
        field('Medienformat (CUPS)', media),
        field('Maximale Drucke pro Foto (0 = unbegrenzt)', limit),
        h('label', { class: 'flex items-center gap-3 my-3' }, autoPrint, 'Automatisch nach jeder Aufnahme drucken'),
        h('div', { class: 'flex items-center gap-3' }, saveBtn, saveMsg),
      ),
      h(
        'div',
        { class: cls.card },
        h(
          'div',
          { class: 'flex items-center justify-between mb-2' },
          h('h2', { class: 'font-semibold' }, 'Druck-Warteschlange'),
          h('button', { class: cls.ghost, onclick: () => void loadJobs() }, 'Aktualisieren'),
        ),
        jobsBox,
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
