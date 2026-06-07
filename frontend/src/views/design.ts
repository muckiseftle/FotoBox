import { api, type EmailSettings, type Theme } from '../api';
import { h, clear, cls, adminPage } from '../ui';
import { applyTheme, setBrandColor } from '../theme';

function field(label: string, input: HTMLElement): HTMLElement {
  return h(
    'label',
    { class: 'block' },
    h('span', { class: 'block text-sm text-slate-300 mb-1' }, label),
    input,
  );
}

export async function renderDesign(app: HTMLElement): Promise<void> {
  clear(app);
  let t: Theme;
  let email: EmailSettings;
  try {
    [t, email] = await Promise.all([api.getTheme(), api.getEmailSettings()]);
  } catch (e) {
    app.append(adminPage('design', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }

  // ---- Design ----
  const color = h('input', { type: 'color', value: t.primary, class: 'w-14 h-10 rounded' }) as HTMLInputElement;
  color.addEventListener('input', () => setBrandColor(color.value)); // live preview
  const title = h('input', { type: 'text', value: t.title, placeholder: 'Event-Titel', class: cls.input }) as HTMLInputElement;
  const subtitle = h('input', { type: 'text', value: t.subtitle, placeholder: 'Untertitel', class: cls.input }) as HTMLInputElement;

  const logoPreview = h('img', {
    class: 'h-12 w-auto rounded bg-slate-800 ' + (t.logo_url ? '' : 'hidden'),
    src: t.logo_url || '',
    alt: 'Logo',
  }) as HTMLImageElement;
  const logoInput = h('input', { type: 'file', accept: 'image/*', class: 'hidden' }) as HTMLInputElement;
  const designMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  logoInput.addEventListener('change', async () => {
    const f = logoInput.files?.[0];
    if (!f) return;
    try {
      const r = await api.uploadLogo(f);
      t.logo_url = r.logo_url;
      logoPreview.src = r.logo_url + '?t=' + Date.now();
      logoPreview.classList.remove('hidden');
      designMsg.textContent = 'Logo hochgeladen ✓';
    } catch (e) {
      designMsg.textContent = e instanceof Error ? e.message : 'Fehler';
    }
    logoInput.value = '';
  });

  const saveDesign = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          await api.putTheme({
            primary: color.value,
            logo_url: t.logo_url,
            title: title.value,
            subtitle: subtitle.value,
            kiosk_bg_url: t.kiosk_bg_url,
          });
          await applyTheme();
          designMsg.textContent = 'Gespeichert ✓';
          setTimeout(() => (designMsg.textContent = ''), 2000);
        } catch (e) {
          designMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'Design speichern',
  );

  // ---- E-mail ----
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
          emailMsg.textContent = 'Gespeichert ✓';
          pass.value = '';
          setTimeout(() => (emailMsg.textContent = ''), 2000);
        } catch (e) {
          emailMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'E-Mail-Einstellungen speichern',
  );

  app.append(
    adminPage(
      'design',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Design & Teilen'),
      h(
        'div',
        { class: cls.card + ' mb-6 space-y-3' },
        h('h2', { class: 'font-semibold' }, 'Design'),
        h('label', { class: 'flex items-center gap-3' }, h('span', { class: 'text-sm text-slate-300' }, 'Primärfarbe'), color),
        field('Event-Titel', title),
        field('Untertitel', subtitle),
        h(
          'div',
          { class: 'flex items-center gap-3 flex-wrap' },
          logoPreview,
          h('button', { class: cls.ghost, onclick: () => logoInput.click() }, 'Logo hochladen'),
          logoInput,
        ),
        h('div', { class: 'flex items-center gap-3' }, saveDesign, designMsg),
      ),
      h(
        'div',
        { class: cls.card + ' space-y-3' },
        h('h2', { class: 'font-semibold' }, 'E-Mail-Versand (SMTP)'),
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
