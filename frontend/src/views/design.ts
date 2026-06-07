import { api, type Theme } from '../api';
import { h, clear, cls, adminPage } from '../ui';
import { applyTheme, setBrandColor, setBackgroundColor } from '../theme';

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
  try {
    t = await api.getTheme();
  } catch (e) {
    app.append(adminPage('design', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }

  // ---- Colours ----
  const color = h('input', { type: 'color', value: t.primary, class: 'w-14 h-10 rounded' }) as HTMLInputElement;
  color.addEventListener('input', () => setBrandColor(color.value)); // live preview
  const bgColor = h('input', { type: 'color', value: t.background, class: 'w-14 h-10 rounded' }) as HTMLInputElement;
  bgColor.addEventListener('input', () => setBackgroundColor(bgColor.value)); // live preview

  const title = h('input', { type: 'text', value: t.title, placeholder: 'Event-Titel', class: cls.input }) as HTMLInputElement;
  const subtitle = h('input', { type: 'text', value: t.subtitle, placeholder: 'Untertitel', class: cls.input }) as HTMLInputElement;

  // ---- Logo ----
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
            background: bgColor.value,
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

  // ---- Kiosk background image (preview off / on-demand) ----
  const bgPreview = h('img', {
    class: 'h-20 rounded-lg bg-slate-800 ' + (t.kiosk_bg_url ? '' : 'hidden'),
    src: t.kiosk_bg_url ? t.kiosk_bg_url + '?t=' + Date.now() : '',
    alt: 'Hintergrund',
  }) as HTMLImageElement;
  const bgInput = h('input', { type: 'file', accept: 'image/*', class: 'hidden' }) as HTMLInputElement;
  const bgMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  bgInput.addEventListener('change', async () => {
    const f = bgInput.files?.[0];
    if (!f) return;
    try {
      const r = await api.uploadKioskBg(f);
      t.kiosk_bg_url = r.kiosk_bg_url;
      bgPreview.src = r.kiosk_bg_url + '?t=' + Date.now();
      bgPreview.classList.remove('hidden');
      bgMsg.textContent = 'Hochgeladen ✓';
    } catch (e) {
      bgMsg.className = 'text-sm text-red-400 min-h-[1.25rem]';
      bgMsg.textContent = e instanceof Error ? e.message : 'Fehler';
    }
    bgInput.value = '';
  });

  app.append(
    adminPage(
      'design',
      h('h1', { class: 'text-xl font-bold mb-4' }, 'Design'),
      h(
        'div',
        { class: cls.card + ' mb-6 space-y-3' },
        h('h2', { class: 'font-semibold' }, 'Farben & Texte'),
        h('label', { class: 'flex items-center gap-3' }, h('span', { class: 'text-sm text-slate-300' }, 'Akzentfarbe'), color),
        h('label', { class: 'flex items-center gap-3' }, h('span', { class: 'text-sm text-slate-300' }, 'Hintergrundfarbe'), bgColor),
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
        h('h2', { class: 'font-semibold' }, 'Kiosk-Hintergrundbild'),
        h('p', { class: 'text-sm text-slate-400' }, 'Wird im Kiosk angezeigt, wenn die Live-Vorschau „aus" oder „erst beim Auslösen" ist (Vorschaumodus unter Kamera).'),
        h(
          'div',
          { class: 'flex items-center gap-3 flex-wrap' },
          bgPreview,
          h('button', { class: cls.ghost, onclick: () => bgInput.click() }, 'Bild hochladen'),
          bgInput,
          bgMsg,
        ),
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
