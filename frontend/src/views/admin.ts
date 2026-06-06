import { api, type Status } from '../api';
import { h, clear, cls, adminPage } from '../ui';

export async function renderAdmin(app: HTMLElement): Promise<void> {
  clear(app);
  let status: Status;
  try {
    status = await api.status();
  } catch (e) {
    app.append(
      adminPage('dashboard', h('p', { class: 'text-red-400' }, e instanceof Error ? e.message : 'Fehler')),
    );
    return;
  }

  const cam = status.camera;
  const stateColor =
    cam.state.state === 'error'
      ? 'text-red-400'
      : cam.state.state === 'disconnected'
        ? 'text-amber-400'
        : 'text-emerald-400';

  app.append(
    adminPage(
      'dashboard',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Dashboard'),
      h(
        'div',
        { class: 'grid gap-4 sm:grid-cols-2 lg:grid-cols-3' },
        statCard('Kamera', [
          row('Modell', cam.info.model),
          row('Backend', cam.info.backend),
          row('Status', h('span', { class: stateColor }, cam.state.state)),
        ]),
        statCard('Event', [
          row('Name', status.event_name || '—'),
          row('Fotos', String(status.photo_count)),
        ]),
        statCard('System', [
          row('Version', status.version),
          row('Drucker erkannt', String(status.printers.length)),
        ]),
      ),
      h(
        'p',
        { class: 'mt-6 text-sm text-slate-400' },
        'Wähle links einen Bereich, um Kamera, Chroma-Key, Druck, Design oder Komfort einzustellen.',
      ),
    ),
  );
}

function statCard(title: string, rows: Node[]): HTMLElement {
  return h(
    'div',
    { class: cls.card },
    h('h2', { class: 'text-sm uppercase tracking-wide text-slate-400 mb-3' }, title),
    h('div', { class: 'space-y-2' }, ...rows),
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
