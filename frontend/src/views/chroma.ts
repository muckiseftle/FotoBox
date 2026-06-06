import { api, type ChromaKey, type Region } from '../api';
import { h, clear, cls, adminPage } from '../ui';

function hex(c: [number, number, number]): string {
  return '#' + c.map((v) => v.toString(16).padStart(2, '0')).join('');
}
function rgb(hexStr: string): [number, number, number] {
  return [
    parseInt(hexStr.slice(1, 3), 16),
    parseInt(hexStr.slice(3, 5), 16),
    parseInt(hexStr.slice(5, 7), 16),
  ];
}
function debounce(fn: () => void, ms: number): () => void {
  let t: number | undefined;
  return () => {
    clearTimeout(t);
    t = setTimeout(fn, ms) as unknown as number;
  };
}

export async function renderChroma(app: HTMLElement): Promise<void> {
  clear(app);
  let key: ChromaKey;
  try {
    key = await api.getChroma();
  } catch (e) {
    app.append(adminPage('chroma', h('p', { class: 'text-red-400' }, String(e))));
    return;
  }

  let region: Region = { x: 0.3, y: 0.3, w: 0.4, h: 0.4 };
  let view: 'composite' | 'matte' = 'composite';
  let lastUrl: string | null = null;
  let stopped = false;

  // ---- Live view with a drag-to-pick region ----
  const live = h('img', {
    class: 'block w-full select-none pointer-events-none',
    src: api.previewStream(),
    alt: 'Live-Vorschau',
  });
  const selBox = h('div', {
    class: 'absolute border-2 border-sky-400 bg-sky-400/20 pointer-events-none',
  });
  const liveWrap = h(
    'div',
    { class: 'relative overflow-hidden rounded-xl bg-black cursor-crosshair touch-none aspect-[4/3]' },
    live,
    selBox,
  );
  const syncSelBox = () => {
    selBox.style.left = `${region.x * 100}%`;
    selBox.style.top = `${region.y * 100}%`;
    selBox.style.width = `${region.w * 100}%`;
    selBox.style.height = `${region.h * 100}%`;
  };
  syncSelBox();

  let dragStart: { x: number; y: number } | null = null;
  const relPos = (ev: PointerEvent) => {
    const r = liveWrap.getBoundingClientRect();
    return {
      x: Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width)),
      y: Math.min(1, Math.max(0, (ev.clientY - r.top) / r.height)),
    };
  };
  liveWrap.addEventListener('pointerdown', (ev) => {
    dragStart = relPos(ev);
    liveWrap.setPointerCapture(ev.pointerId);
  });
  liveWrap.addEventListener('pointermove', (ev) => {
    if (!dragStart) return;
    const p = relPos(ev);
    region = {
      x: Math.min(dragStart.x, p.x),
      y: Math.min(dragStart.y, p.y),
      w: Math.abs(p.x - dragStart.x),
      h: Math.abs(p.y - dragStart.y),
    };
    syncSelBox();
  });
  const endDrag = () => {
    dragStart = null;
  };
  liveWrap.addEventListener('pointerup', endDrag);
  liveWrap.addEventListener('pointercancel', endDrag);

  // ---- WYSIWYG preview (polled so subject motion stays live) ----
  const result = h('img', { class: 'block w-full rounded-xl bg-black aspect-[4/3] object-cover', alt: 'Ergebnis-Vorschau' });
  const refreshPreview = async () => {
    if (stopped) return;
    try {
      const url = await api.chromaPreviewBlob(key, view);
      if (lastUrl) URL.revokeObjectURL(lastUrl);
      lastUrl = url;
      result.src = url;
    } catch {
      /* transient (e.g. no frame yet) */
    }
  };
  const debouncedPreview = debounce(() => void refreshPreview(), 120);
  const timer = setInterval(() => void refreshPreview(), 800);
  window.addEventListener(
    'hashchange',
    () => {
      stopped = true;
      clearInterval(timer);
      if (lastUrl) URL.revokeObjectURL(lastUrl);
    },
    { once: true },
  );
  void refreshPreview();

  // ---- Controls ----
  const colorInput = h('input', { type: 'color', value: hex(key.key_color), class: 'w-12 h-10 rounded' }) as HTMLInputElement;
  colorInput.addEventListener('input', () => {
    key.key_color = rgb(colorInput.value);
    debouncedPreview();
  });

  const enabled = h('input', { type: 'checkbox', class: 'w-5 h-5 accent-sky-500' }) as HTMLInputElement;
  enabled.checked = key.enabled;
  enabled.addEventListener('change', () => {
    key.enabled = enabled.checked;
  });

  const sliders: Array<{ set: (v: number) => void }> = [];
  const makeSlider = (
    label: string,
    min: number,
    max: number,
    step: number,
    get: () => number,
    set: (v: number) => void,
  ) => {
    const valueLabel = h('span', { class: 'text-slate-400 tabular-nums' }, get().toFixed(step < 1 ? 2 : 0));
    const input = h('input', {
      type: 'range',
      min,
      max,
      step,
      value: get(),
      class: 'w-full accent-sky-500',
    }) as HTMLInputElement;
    input.addEventListener('input', () => {
      const v = parseFloat(input.value);
      set(v);
      valueLabel.textContent = v.toFixed(step < 1 ? 2 : 0);
      debouncedPreview();
    });
    sliders.push({
      set: (v) => {
        input.value = String(v);
        valueLabel.textContent = v.toFixed(step < 1 ? 2 : 0);
      },
    });
    return h(
      'label',
      { class: 'block' },
      h('div', { class: 'flex justify-between text-sm mb-1' }, h('span', {}, label), valueLabel),
      input,
    );
  };

  const tolSlider = makeSlider('Toleranz', 0, 150, 1, () => key.tolerance, (v) => (key.tolerance = v));
  const softSlider = makeSlider('Weichheit (Kanten)', 1, 90, 1, () => key.softness, (v) => (key.softness = v));
  const despillSlider = makeSlider('Despill (Grünstich)', 0, 1, 0.01, () => key.despill, (v) => (key.despill = v));

  const syncControls = () => {
    colorInput.value = hex(key.key_color);
    sliders[0]?.set(key.tolerance);
    sliders[1]?.set(key.softness);
    sliders[2]?.set(key.despill);
  };

  // Lighting feedback
  const evenBar = h('div', { class: 'h-2 rounded-full bg-slate-700 overflow-hidden' }, h('div', { class: 'h-full w-0 bg-emerald-500 transition-all' }));
  const evenMsg = h('p', { class: 'text-sm text-slate-300 mt-2 min-h-[1.25rem]' });
  const showEvenness = (score: number, message: string) => {
    const bar = evenBar.firstElementChild as HTMLElement;
    bar.style.width = `${Math.round(score * 100)}%`;
    bar.className =
      'h-full transition-all ' +
      (score > 0.8 ? 'bg-emerald-500' : score > 0.55 ? 'bg-amber-500' : 'bg-red-500');
    evenMsg.textContent = message;
  };

  const calibrateBtn = h(
    'button',
    {
      class: cls.button + ' w-full',
      onclick: async () => {
        try {
          const cal = await api.calibrate(region);
          key.key_color = cal.key_color;
          key.tolerance = cal.tolerance;
          key.softness = cal.softness;
          key.enabled = true;
          enabled.checked = true;
          syncControls();
          showEvenness(cal.evenness, cal.message);
          void refreshPreview();
        } catch (e) {
          evenMsg.textContent = e instanceof Error ? e.message : 'Kalibrierung fehlgeschlagen';
        }
      },
    },
    '1-Klick automatisch kalibrieren',
  );

  const viewToggle = h(
    'button',
    {
      class: cls.ghost + ' w-full',
      onclick: () => {
        view = view === 'composite' ? 'matte' : 'composite';
        void refreshPreview();
      },
    },
    'Ansicht: Ergebnis / Maske',
  );

  const saveMsg = h('span', { class: 'text-sm text-emerald-400 min-h-[1.25rem]' });
  const saveBtn = h(
    'button',
    {
      class: cls.button,
      onclick: async () => {
        try {
          await api.putChroma(key);
          saveMsg.textContent = 'Gespeichert ✓';
          setTimeout(() => (saveMsg.textContent = ''), 2000);
        } catch (e) {
          saveMsg.textContent = e instanceof Error ? e.message : 'Fehler';
        }
      },
    },
    'Speichern',
  );

  // ---- Backgrounds ----
  const bgGrid = h('div', { class: 'grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-2' });
  const fileInput = h('input', { type: 'file', accept: 'image/*', class: 'hidden' }) as HTMLInputElement;
  fileInput.addEventListener('change', async () => {
    const file = fileInput.files?.[0];
    if (!file) return;
    try {
      const { ulid } = await api.uploadBackground(file.name, file);
      await api.selectBackground(ulid);
      await loadBackgrounds();
      void refreshPreview();
    } catch (e) {
      saveMsg.textContent = e instanceof Error ? e.message : 'Upload-Fehler';
    }
    fileInput.value = '';
  });

  const loadBackgrounds = async () => {
    const data = await api.listBackgrounds();
    bgGrid.replaceChildren();
    for (const bg of data.backgrounds) {
      const tile = h(
        'div',
        {
          class:
            'relative aspect-square rounded-lg overflow-hidden cursor-pointer ring-2 ' +
            (bg.selected ? 'ring-sky-400' : 'ring-transparent hover:ring-slate-600'),
          onclick: async () => {
            await api.selectBackground(bg.ulid);
            await loadBackgrounds();
            void refreshPreview();
          },
        },
        h('img', { class: 'w-full h-full object-cover', src: bg.url, alt: bg.name }),
        h(
          'button',
          {
            class: 'absolute top-1 right-1 bg-black/60 rounded-full w-6 h-6 text-white text-sm leading-none',
            title: 'Löschen',
            onclick: async (e: Event) => {
              e.stopPropagation();
              await api.deleteBackground(bg.ulid);
              await loadBackgrounds();
              void refreshPreview();
            },
          },
          '×',
        ),
      );
      bgGrid.append(tile);
    }
    const uploadTile = h(
      'button',
      {
        class:
          'aspect-square rounded-lg border-2 border-dashed border-slate-600 text-slate-400 ' +
          'hover:border-sky-500 hover:text-sky-400 grid place-items-center text-3xl',
        onclick: () => fileInput.click(),
      },
      '+',
    );
    bgGrid.append(uploadTile);
  };
  await loadBackgrounds();

  // ---- Layout ----
  app.append(
    adminPage(
      'chroma',
      h(
        'div',
        { class: 'flex items-center justify-between mb-4' },
        h('h1', { class: 'text-xl font-bold' }, 'Chroma-Key Kalibrierung'),
        h('label', { class: 'flex items-center gap-2 text-sm' }, enabled, 'Aktiv'),
      ),
      h(
        'div',
        { class: 'grid gap-6 lg:grid-cols-2' },
        // Left: live + region pick + calibrate + lighting
        h(
          'div',
          { class: cls.card + ' space-y-3' },
          h('h2', { class: 'font-semibold' }, '1. Hintergrund-Fläche markieren'),
          h('p', { class: 'text-sm text-slate-400' }, 'Ziehe ein Rechteck über das Tuch / die Wand, dann kalibriere automatisch.'),
          liveWrap,
          calibrateBtn,
          h('div', {}, h('div', { class: 'text-sm mb-1 text-slate-300' }, 'Ausleuchtung'), evenBar, evenMsg),
        ),
        // Right: WYSIWYG + controls
        h(
          'div',
          { class: cls.card + ' space-y-3' },
          h('h2', { class: 'font-semibold' }, '2. Ergebnis & Feinabstimmung'),
          result,
          viewToggle,
          h('label', { class: 'flex items-center gap-3 text-sm' }, 'Key-Farbe', colorInput),
          tolSlider,
          softSlider,
          despillSlider,
          h('div', { class: 'flex items-center gap-3 pt-1' }, saveBtn, saveMsg),
        ),
      ),
      h(
        'div',
        { class: cls.card + ' mt-6' },
        h('h2', { class: 'font-semibold mb-1' }, '3. Hintergrund wählen'),
        h('p', { class: 'text-sm text-slate-400 mb-3' }, 'Hochladen mit „+", auswählen per Klick.'),
        bgGrid,
        fileInput,
      ),
      h('a', { href: '#/admin', class: 'inline-block mt-6 ' + cls.ghost }, 'Zurück zum Admin'),
    ),
  );
}
