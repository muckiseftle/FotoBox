// Tiny DOM helpers and shared style tokens — no framework, minimal payload.

import { t } from './i18n';
import { api } from './api';

type Attrs = Record<string, unknown>;

export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  ...children: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (value == null || value === false) continue;
    if (key === 'class') el.className = String(value);
    // NOTE: deliberately no `innerHTML` sink here — all text goes through
    // `textContent`/child nodes to keep the DOM builder XSS-safe.
    else if (key.startsWith('on') && typeof value === 'function') {
      el.addEventListener(key.slice(2).toLowerCase(), value as EventListener);
    } else if (value === true) el.setAttribute(key, '');
    else el.setAttribute(key, String(value));
  }
  el.append(...children);
  return el;
}

export function clear(node: HTMLElement): void {
  node.replaceChildren();
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// Shared Tailwind class tokens. Accent colour comes from the themeable
// `--color-brand` CSS variable (Tailwind v4 `brand` palette).
export const cls = {
  input:
    'w-full rounded-xl bg-slate-800/80 border border-slate-700 px-4 py-3 text-base ' +
    'outline-none focus:border-brand focus:ring-2 focus:ring-brand/40',
  button:
    'rounded-xl bg-brand hover:brightness-110 active:brightness-95 text-slate-950 font-semibold ' +
    'px-5 py-3 transition disabled:opacity-40 disabled:cursor-not-allowed',
  ghost:
    'rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-100 font-medium px-5 py-3 transition-colors',
  card: 'rounded-2xl bg-slate-900/70 border border-slate-800 p-5',
};

/** Page chrome (top nav) used by the gallery and admin views. */
export function page(active: 'gallery' | 'admin', ...content: (Node | string)[]): HTMLElement {
  const link = (href: string, label: string, key: string) =>
    h(
      'a',
      {
        href,
        class:
          'px-4 py-2 rounded-lg text-sm font-medium ' +
          (active === key ? 'bg-brand text-slate-950' : 'text-slate-300 hover:bg-slate-800'),
      },
      label,
    );

  return h(
    'div',
    { class: 'min-h-full max-w-6xl mx-auto px-4 pb-16' },
    h(
      'header',
      { class: 'flex items-center gap-2 py-4 sticky top-0 bg-bg/90 backdrop-blur z-10' },
      h('a', { href: '#/', class: 'mr-auto text-lg font-bold tracking-tight' }, 'SnapStation'),
      link('#/gallery', t('gallery'), 'gallery'),
      link('#/admin', t('admin'), 'admin'),
    ),
    ...content,
  );
}

/** Admin section identifiers for the sidebar. */
export type AdminKey =
  | 'dashboard'
  | 'setup'
  | 'camera'
  | 'capture'
  | 'collage'
  | 'chroma'
  | 'print'
  | 'share'
  | 'design'
  | 'kiosk'
  | 'comfort';

/** Admin layout with a left sidebar menu (responsive: sidebar on desktop,
 *  horizontal scroll row on mobile). */
export function adminPage(active: AdminKey, ...content: (Node | string)[]): HTMLElement {
  const groups: { title: string; items: { key: AdminKey; href: string; label: string }[] }[] = [
    {
      title: 'Allgemein',
      items: [
        { key: 'dashboard', href: '#/admin', label: 'Übersicht' },
        { key: 'setup', href: '#/setup', label: 'Allgemein' },
      ],
    },
    {
      title: 'Aufnahme',
      items: [
        { key: 'camera', href: '#/camera', label: 'Kamera' },
        { key: 'capture', href: '#/capture', label: 'Aufnahme' },
        { key: 'collage', href: '#/collage', label: 'Collage' },
        { key: 'chroma', href: '#/chroma', label: 'Chroma-Key' },
      ],
    },
    {
      title: 'Teilen & Druck',
      items: [
        { key: 'print', href: '#/print', label: 'Drucken' },
        { key: 'share', href: '#/share', label: 'Teilen' },
      ],
    },
    {
      title: 'Anzeige & Komfort',
      items: [
        { key: 'design', href: '#/design', label: 'Design' },
        { key: 'kiosk', href: '#/kiosk-settings', label: 'Kiosk' },
        { key: 'comfort', href: '#/settings', label: 'Diashow & Sicherung' },
      ],
    },
  ];
  const itemCls = (on: boolean) =>
    'block px-3 py-2 rounded-lg text-sm font-medium whitespace-nowrap ' +
    (on ? 'bg-brand text-slate-950' : 'text-slate-300 hover:bg-slate-800');
  const groupTitleCls = 'px-3 pt-3 pb-1 text-[10px] font-semibold uppercase tracking-wider text-slate-500 shrink-0';
  const footerItem = 'text-left block px-3 py-2 rounded-lg text-sm text-slate-400 hover:bg-slate-800';

  const sidebar = h(
    'aside',
    {
      class:
        cls.card +
        ' md:w-60 shrink-0 md:sticky md:top-4 md:h-[calc(100vh-2rem)] flex md:flex-col gap-2 ' +
        'overflow-x-auto md:overflow-y-auto md:overflow-x-visible',
    },
    h('a', { href: '#/', class: 'text-lg font-bold tracking-tight px-1 mb-1 shrink-0' }, 'SnapStation'),
    h(
      'nav',
      { class: 'flex md:flex-col gap-1' },
      ...groups.flatMap((g) => [
        h('div', { class: groupTitleCls }, g.title),
        ...g.items.map((it) =>
          h('a', { href: it.href, class: itemCls(active === it.key) }, it.label),
        ),
      ]),
    ),
    h(
      'div',
      { class: 'flex md:flex-col gap-1 md:mt-auto shrink-0' },
      h('a', { href: '#/', class: footerItem }, 'Zur Kiosk-Ansicht'),
      h(
        'button',
        {
          class: footerItem,
          onclick: async () => {
            await api.logout().catch(() => {});
            location.hash = '#/';
          },
        },
        'Abmelden',
      ),
    ),
  );

  return h(
    'div',
    { class: 'min-h-full max-w-6xl mx-auto px-4 py-4 flex flex-col md:flex-row gap-4' },
    sidebar,
    h('main', { class: 'flex-1 min-w-0 pb-12' }, ...content),
  );
}
