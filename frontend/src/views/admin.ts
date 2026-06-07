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
      buildUpdateCard(status.version),
      h(
        'p',
        { class: 'mt-6 text-sm text-slate-400' },
        'Wähle links einen Bereich, um Kamera, Chroma-Key, Druck, Design oder Komfort einzustellen.',
      ),
    ),
  );
}

/** A "Software update" card: check GitHub for a newer release and apply it. */
function buildUpdateCard(currentVersion: string): HTMLElement {
  const msg = h('p', { class: 'text-sm text-slate-300 min-h-[1.25rem]' });
  const actions = h('div', { class: 'flex flex-wrap items-center gap-3 mt-2' });

  const checkBtn = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        msg.className = 'text-sm text-slate-300 min-h-[1.25rem]';
        msg.textContent = 'Suche nach Updates …';
        actions.replaceChildren(checkBtn);
        try {
          const r = await api.updateCheck();
          if (!r.update_available) {
            msg.className = 'text-sm text-emerald-400 min-h-[1.25rem]';
            msg.textContent = `Aktuell: v${r.current} — keine Updates verfügbar.`;
            return;
          }
          msg.className = 'text-sm text-amber-300 min-h-[1.25rem]';
          msg.textContent = `Update verfügbar: v${r.current} → v${r.latest}`;
          if (r.can_self_update) {
            const applyBtn = h(
              'button',
              {
                class: cls.button,
                onclick: async () => {
                  msg.className = 'text-sm text-slate-300 min-h-[1.25rem]';
                  msg.textContent = 'Lade & installiere Update … (einen Moment)';
                  try {
                    const a = await api.updateApply();
                    msg.className = 'text-sm text-emerald-400 min-h-[1.25rem]';
                    msg.textContent = `Aktualisiert auf v${a.updated_to}. Bitte SnapStation neu starten.`;
                  } catch (e) {
                    msg.className = 'text-sm text-red-400 min-h-[1.25rem]';
                    msg.textContent = e instanceof Error ? e.message : 'Update fehlgeschlagen';
                  }
                },
              },
              'Jetzt aktualisieren',
            );
            actions.replaceChildren(checkBtn, applyBtn);
          } else {
            actions.replaceChildren(
              checkBtn,
              h('a', { class: cls.ghost, href: r.release_url, target: '_blank' }, 'Zum Download'),
            );
          }
        } catch (e) {
          msg.className = 'text-sm text-red-400 min-h-[1.25rem]';
          msg.textContent = e instanceof Error ? e.message : 'Update-Prüfung fehlgeschlagen';
        }
      },
    },
    'Nach Updates suchen',
  );
  actions.append(checkBtn);

  return h(
    'div',
    { class: cls.card + ' mt-6' },
    h('h2', { class: 'font-semibold mb-1' }, 'Software-Update'),
    h('p', { class: 'text-sm text-slate-400 mb-1' }, `Installierte Version: v${currentVersion}`),
    msg,
    actions,
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
