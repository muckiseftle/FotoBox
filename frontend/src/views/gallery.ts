import { api, type Photo } from '../api';
import { h, clear, cls, page } from '../ui';
import { t } from '../i18n';

export async function renderGallery(app: HTMLElement): Promise<void> {
  clear(app);
  let photos: Photo[] = [];
  try {
    photos = await api.photos(120, 0);
  } catch (e) {
    app.append(
      page('gallery', h('p', { class: 'text-red-400' }, e instanceof Error ? e.message : 'Fehler')),
    );
    return;
  }

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
          onclick: () => openLightbox(app, photo),
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
      h('h1', { class: 'text-xl font-bold mb-4' }, `${t('gallery')} · ${photos.length} ${t('photos')}`),
      grid,
    ),
  );
}

function openLightbox(app: HTMLElement, photo: Photo): void {
  const close = () => box.remove();
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
      h(
        'div',
        { class: 'flex items-center gap-4' },
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
      ),
    ),
  );
  app.append(box);
}
