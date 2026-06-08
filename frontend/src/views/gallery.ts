import { api, type Photo, type KioskSettings, setPublicHost } from '../api';
import { h, clear, cls, page } from '../ui';
import { t } from '../i18n';

export async function renderGallery(app: HTMLElement): Promise<void> {
  clear(app);
  let photos: Photo[] = [];
  let kcfg: KioskSettings | null = null;
  try {
    [photos, kcfg] = await Promise.all([api.photos(120, 0), api.getKiosk().catch(() => null)]);
    await api
      .getNetwork()
      .then((n) => setPublicHost(n.selected))
      .catch(() => {});
  } catch (e) {
    app.append(
      page('gallery', h('p', { class: 'text-red-400' }, e instanceof Error ? e.message : 'Fehler')),
    );
    return;
  }

  const allowDelete = kcfg?.allow_delete_gallery ?? false;

  const grid = h('div', {
    class: 'grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2',
  });

  if (photos.length === 0) {
    grid.append(h('p', { class: 'text-slate-400 col-span-full' }, t('noPhotos')));
  }

  for (const photo of photos) {
    grid.append(
      h(
        'button',
        {
          class: 'aspect-square overflow-hidden rounded-xl bg-slate-800',
          onclick: () => openLightbox(app, photo, allowDelete),
        },
        h('img', {
          class: 'w-full h-full object-cover hover:scale-105 transition-transform',
          loading: 'lazy',
          src: api.thumbUrl(photo.token),
          alt: 'Foto',
        }),
      ),
    );
  }

  app.append(
    page(
      'gallery',
      h(
        'div',
        { class: 'flex items-center justify-between gap-3 mb-4' },
        h('h1', { class: 'text-xl font-bold' }, `${t('gallery')} · ${photos.length} ${t('photos')}`),
        h('a', { class: cls.button, href: '#/' }, '← ' + t('home')),
      ),
      grid,
    ),
  );

  // Auto-return to the kiosk after a period of inactivity (0 = off).
  const idle = kcfg?.gallery_idle_seconds ?? 0;
  if (idle > 0) {
    let timer: number | undefined;
    const reset = () => {
      clearTimeout(timer);
      timer = window.setTimeout(() => {
        location.hash = '#/';
      }, idle * 1000);
    };
    const events = ['pointerdown', 'keydown', 'pointermove'];
    events.forEach((e) => window.addEventListener(e, reset, { passive: true }));
    reset();
    window.addEventListener(
      'hashchange',
      () => {
        clearTimeout(timer);
        events.forEach((e) => window.removeEventListener(e, reset));
      },
      { once: true },
    );
  }
}

function openLightbox(app: HTMLElement, photo: Photo, allowDelete: boolean): void {
  const close = () => box.remove();
  const actions = [
    h(
      'a',
      {
        class: cls.ghost,
        href: api.photoUrl(photo.token),
        download: `${photo.ulid}.jpg`,
        onclick: (e: Event) => e.stopPropagation(),
      },
      t('download'),
    ),
    h('img', {
      class: 'w-24 h-24 rounded-lg bg-white p-1',
      src: api.qrUrl(photo.token),
      alt: 'QR-Code',
      onclick: (e: Event) => e.stopPropagation(),
    }),
  ];
  if (allowDelete) {
    actions.push(
      h(
        'button',
        {
          class: 'rounded-xl bg-red-600/80 hover:bg-red-600 text-white font-medium px-5 py-3 transition',
          onclick: async (e: Event) => {
            e.stopPropagation();
            if (!confirm(t('confirmDelete'))) return;
            try {
              await api.deletePhoto(photo.token);
              close();
              void renderGallery(app); // refresh the grid
            } catch {
              /* ignore */
            }
          },
        },
        t('delete'),
      ),
    );
  }
  const box = h(
    'div',
    {
      class: 'fixed inset-0 z-50 bg-black/90 grid place-items-center p-4',
      onclick: close,
    },
    h(
      'div',
      { class: 'flex flex-col items-center gap-4' },
      h('img', {
        class: 'max-h-[80vh] max-w-full rounded-2xl shadow-2xl',
        src: api.photoUrl(photo.token),
        alt: 'Foto',
      }),
      h('div', { class: 'flex items-center gap-4 flex-wrap justify-center' }, ...actions),
    ),
  );
  app.append(box);
}
