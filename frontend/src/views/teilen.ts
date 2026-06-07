import { api, type EmailSettings } from '../api';
import { h, clear, cls, adminPage } from '../ui';

function field(label: string, input: HTMLElement): HTMLElement {
  return h(
    'label',
    { class: 'block' },
    h('span', { class: 'block text-sm text-slate-300 mb-1' }, label),
    input,
  );
}

export async function renderShare(app: HTMLElement): Promise<void> {
  clear(app);
  let email: EmailSettings;
  try {
    email = await api.getEmailSettings();
  } catch (e) {
    app.append(adminPage('share', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }

  const enabled = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-brand' }) as HTMLInputElement;
  enabled.checked = email.enabled;
  const host = h('input', { type: 'text', value: email.host, placeholder: 'smtp.example.com', class: cls.input }) as HTMLInputElement;
  const port = h('input', { type: 'number', value: email.port, min: 1, max: 65535, class: cls.input }) as HTMLInputElement;
  const user = h('input', { type: 'text', value: email.user, placeholder: 'Benutzername', class: cls.input }) as HTMLInputElement;
  const from = h('input', { type: 'text', value: email.from, placeholder: 'foto@example.com', class: cls.input }) as HTMLInputElement;
  const pass = h('input', {
    type: 'password',
    placeholder: email.has_password ? '•••••• (gespeichert — leer = beibehalten)' : 'Passwort',
    class: cls.input,
  }) as HTMLInputElement;
  const emailMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const saveEmail = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          await api.setEmailSettings({
            enabled: enabled.checked,
            host: host.value,
            port: Number(port.value) || 587,
            user: user.value,
            from: from.value,
            password: pass.value || undefined,
          });
          emailMsg.className = 'text-sm text-emerald-400 min-h-[1.25rem]';
          emailMsg.textContent = 'Gespeichert ✓';
          pass.value = '';
          setTimeout(() => (emailMsg.textContent = ''), 2000);
        } catch (e) {
          emailMsg.className = 'text-sm text-red-400 min-h-[1.25rem]';
          emailMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'E-Mail-Einstellungen speichern',
  );

  app.append(
    adminPage(
      'share',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Teilen'),
      h(
        'div',
        { class: cls.card + ' space-y-3' },
        h('h2', { class: 'font-semibold' }, 'E-Mail-Versand (SMTP)'),
        h('p', { class: 'text-sm text-slate-400' }, 'Gäste können Fotos per E-Mail erhalten. QR-Code-Download steht immer zur Verfügung (im Kiosk ein-/ausblendbar).'),
        h('label', { class: 'flex items-center gap-3' }, enabled, 'E-Mail-Versand aktivieren'),
        field('SMTP-Server', host),
        field('Port', port),
        field('Benutzer', user),
        field('Absender (From)', from),
        field('Passwort', pass),
        h('div', { class: 'flex items-center gap-3' }, saveEmail, emailMsg),
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
