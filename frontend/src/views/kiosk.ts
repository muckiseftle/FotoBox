import { api, type Photo, COLLAGE_SHOTS } from '../api';
import { h, clear, cls, sleep } from '../ui';
import { theme } from '../theme';
import { t, getLang, setLang } from '../i18n';

// Fullscreen guest experience: live view, countdown, capture, result.
export function renderKiosk(app: HTMLElement): void {
  clear(app);

  // Background image (shown when the live preview is off / on-demand).
  const bgImg = h('img', {
    class: 'absolute inset-0 w-full h-full object-cover hidden',
    alt: '',
  }) as HTMLImageElement;

  const preview = h('img', {
    class: 'absolute inset-0 w-full h-full object-cover',
    alt: 'Live-Vorschau',
    src: api.previewStream(),
  }) as HTMLImageElement;

  // Live-preview mode: 'live' (always), 'on_demand' (only while shooting), 'off'.
  let mode: 'live' | 'on_demand' | 'off' = 'live';
  const kioskBg = theme().kiosk_bg_url;
  const setLive = (on: boolean) => {
    if (on) {
      if (!preview.src.includes('/preview/stream')) preview.src = api.previewStream();
      preview.classList.remove('hidden');
      bgImg.classList.add('hidden');
    } else {
      preview.src = '';
      preview.classList.add('hidden');
      if (kioskBg) {
        bgImg.src = kioskBg;
        bgImg.classList.remove('hidden');
      } else {
        bgImg.classList.add('hidden');
      }
    }
  };
  const applyMode = () => setLive(mode === 'live');

  const overlay = h('div', { class: 'absolute inset-0 grid place-items-center pointer-events-none' });

  const shutter = h(
    'button',
    {
      class:
        'w-24 h-24 rounded-full bg-white/90 active:scale-95 transition-transform ' +
        'shadow-2xl ring-4 ring-brand/60 shrink-0',
      'aria-label': t('shutter'),
    },
    h('span', { class: 'block w-16 h-16 rounded-full bg-brand mx-auto' }),
  );

  // Branding overlay (logo + event title), driven by the saved theme.
  const th = theme();
  const brand = h(
    'div',
    { class: 'absolute top-4 left-4 flex items-center gap-3 text-white drop-shadow-lg' },
    th.logo_url ? h('img', { src: th.logo_url, alt: '', class: 'h-12 w-auto' }) : '',
    h(
      'div',
      {},
      h('div', { class: 'font-bold leading-tight' }, th.title || ''),
      h('div', { class: 'text-sm opacity-80 leading-tight' }, th.subtitle || ''),
    ),
  );

  const gear = h(
    'a',
    {
      href: '#/admin',
      class: 'text-slate-200/70 hover:text-white text-2xl leading-none',
      'aria-label': 'Admin',
    },
    '⚙',
  );

  const galleryBtn = h(
    'a',
    {
      href: '#/gallery',
      class:
        'flex items-center gap-2 bg-black/40 hover:bg-black/60 text-white text-sm font-medium px-4 py-3 ' +
        'rounded-xl backdrop-blur whitespace-nowrap shrink-0',
      'aria-label': t('gallery'),
    },
    h('img', { src: '/gallery.svg', alt: '', class: 'w-5 h-5' }),
    h('span', {}, t('gallery')),
  );

  // Optional collage button (revealed when collage capture is enabled).
  const collageBtn = h(
    'button',
    {
      class:
        'hidden flex items-center gap-2 bg-black/40 hover:bg-black/60 text-white text-sm font-medium px-4 py-3 ' +
        'rounded-xl backdrop-blur whitespace-nowrap shrink-0',
      'aria-label': 'Collage',
    },
    h('span', { class: 'text-base leading-none' }, '▦'),
    h('span', {}, 'Collage'),
  );

  const langToggle = h(
    'button',
    {
      class: 'text-slate-200/70 hover:text-white text-sm font-semibold',
      'aria-label': 'Sprache / Language',
    },
    getLang().toUpperCase(),
  );
  langToggle.addEventListener('click', () => {
    setLang(getLang() === 'de' ? 'en' : 'de');
    renderKiosk(app);
  });

  const topRight = h(
    'div',
    { class: 'absolute top-4 right-4 flex items-center gap-4' },
    langToggle,
    gear,
  );

  // Bottom bar: gallery + collage buttons flank the (centred) shutter.
  const bottomBar = h(
    'div',
    { class: 'absolute bottom-8 left-1/2 -translate-x-1/2 flex items-center gap-6' },
    galleryBtn,
    shutter,
    collageBtn,
  );

  const stage = h(
    'div',
    { class: 'kiosk relative w-full h-full bg-black overflow-hidden' },
    bgImg,
    preview,
    overlay,
    brand,
    bottomBar,
    topRight,
  );
  app.append(stage);

  let busy = false;
  let resultTimer: number | undefined;

  // Kiosk behaviour (which controls to show, auto-return). Loaded from the admin.
  let kcfg = {
    show_gallery: true,
    show_print: true,
    show_qr: true,
    result_seconds: 0,
    qr_logo: false,
    allow_delete_gallery: false,
    allow_delete_after_capture: false,
    gallery_idle_seconds: 0,
  };
  api
    .getKiosk()
    .then((k) => {
      kcfg = k;
      galleryBtn.classList.toggle('hidden', !k.show_gallery);
    })
    .catch(() => {});

  // Collage capture (multi-shot). Revealed when enabled in the admin.
  let collageCfg = { enabled: false, layout: 'grid2x2', countdown_seconds: 3, review_seconds: 2 };
  api
    .getCollage()
    .then((c) => {
      collageCfg = c;
      collageBtn.classList.toggle('hidden', !c.enabled);
    })
    .catch(() => {});

  // Capture settings (countdown length, beep, selfie mirror) from the admin.
  let capture = { countdown: 3, sound: true, mirror: false };
  let audioCtx: AudioContext | null = null;
  const beep = () => {
    if (!capture.sound) return;
    try {
      audioCtx = audioCtx ?? new AudioContext();
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.type = 'sine';
      osc.frequency.value = 880;
      gain.gain.value = 0.15;
      osc.connect(gain);
      gain.connect(audioCtx.destination);
      osc.start();
      osc.stop(audioCtx.currentTime + 0.12);
    } catch {
      /* audio unavailable */
    }
  };
  api
    .getCamera()
    .then((c) => {
      capture = {
        countdown: c.settings.countdown_seconds,
        sound: c.settings.countdown_sound,
        mirror: c.settings.mirror_preview,
      };
      preview.style.transform = capture.mirror ? 'scaleX(-1)' : '';
      mode = (c.settings.preview_mode as 'live' | 'on_demand' | 'off') || 'live';
      applyMode();
    })
    .catch(() => {});

  const showCountdown = (text: string, sub?: string) => {
    overlay.replaceChildren(
      h(
        'div',
        { class: 'flex flex-col items-center gap-2' },
        sub ? h('div', { class: 'text-white/90 font-semibold text-3xl drop-shadow-lg' }, sub) : '',
        h(
          'div',
          { class: 'text-white font-black drop-shadow-2xl text-[28vh] leading-none animate-pulse' },
          text,
        ),
      ),
    );
  };

  // Briefly show a just-taken collage shot with progress.
  const showShotReview = (photo: Photo, idx: number, total: number) => {
    overlay.replaceChildren(
      h(
        'div',
        { class: 'absolute inset-0 bg-black/70 grid place-items-center p-6' },
        h(
          'div',
          { class: 'flex flex-col items-center gap-3' },
          h('div', { class: 'text-white/90 font-semibold text-2xl' }, `Bild ${idx}/${total}`),
          h('img', {
            class: 'max-h-[60vh] rounded-2xl shadow-2xl',
            src: api.photoUrl(photo.token),
            alt: '',
          }),
        ),
      ),
    );
  };

  const showResult = (photo: Photo) => {
    const printStatus = h('p', { class: 'text-sm min-h-[1.25rem] text-slate-200' });
    const printBtn = h(
      'button',
      {
        class: cls.button,
        onclick: async () => {
          printStatus.className = 'text-sm min-h-[1.25rem] text-slate-200';
          printStatus.textContent = t('printSending');
          try {
            const r = await api.print(photo.token);
            printStatus.className = 'text-sm min-h-[1.25rem] text-emerald-300';
            printStatus.textContent = r.status === 'done' ? t('printed') : t('printing');
          } catch (e) {
            printStatus.className = 'text-sm min-h-[1.25rem] text-red-300';
            printStatus.textContent = e instanceof Error ? e.message : t('printError');
          }
        },
      },
      t('print'),
    );

    overlay.replaceChildren(
      h(
        'div',
        { class: 'pointer-events-auto absolute inset-0 bg-black/80 grid place-items-center p-6' },
        h(
          'div',
          { class: 'max-w-3xl w-full flex flex-col items-center gap-4' },
          h('img', {
            class: 'max-h-[68vh] rounded-2xl shadow-2xl',
            src: api.photoUrl(photo.token),
            alt: 'Aufnahme',
          }),
          h(
            'div',
            { class: 'flex gap-3 flex-wrap justify-center' },
            kcfg.show_print ? printBtn : '',
            h('button', { class: cls.ghost, onclick: () => backToLive() }, t('retake')),
            kcfg.allow_delete_after_capture
              ? h(
                  'button',
                  {
                    class:
                      'rounded-xl bg-red-600/80 hover:bg-red-600 text-white font-medium px-5 py-3 transition',
                    onclick: async () => {
                      try {
                        await api.deletePhoto(photo.token);
                      } catch {
                        /* ignore */
                      }
                      backToLive();
                    },
                  },
                  t('delete'),
                )
              : '',
            kcfg.show_gallery ? h('a', { class: cls.ghost, href: '#/gallery' }, t('toGallery')) : '',
          ),
          printStatus,
          kcfg.show_qr
            ? h(
                'div',
                { class: 'flex items-center gap-3 text-slate-300 text-sm' },
                h('img', {
                  class: 'w-24 h-24 rounded-lg bg-white p-1',
                  src: api.qrUrl(photo.token),
                  alt: 'QR-Code',
                }),
                h('span', {}, t('scanHint')),
              )
            : '',
        ),
      ),
    );

    // Auto-return to the live view after the configured delay (0 = manual).
    if (kcfg.result_seconds > 0) {
      clearTimeout(resultTimer);
      resultTimer = window.setTimeout(() => backToLive(), kcfg.result_seconds * 1000);
    }
  };

  const backToLive = () => {
    clearTimeout(resultTimer);
    overlay.replaceChildren();
    busy = false;
    bottomBar.classList.remove('hidden'); // restore shutter/gallery controls
    applyMode();
  };

  const shoot = async () => {
    if (busy) return;
    busy = true;
    // Hide the shutter/gallery bar so it never overlaps the countdown/result QR.
    bottomBar.classList.add('hidden');
    // On-demand mode: turn the live view on for the countdown.
    if (mode === 'on_demand') setLive(true);
    try {
      const secs = Math.max(0, Math.min(10, capture.countdown));
      for (let i = secs; i >= 1; i--) {
        showCountdown(String(i));
        beep();
        await sleep(800);
      }
      showCountdown('📸');
      const photo = await api.capture();
      showResult(photo);
    } catch (e) {
      overlay.replaceChildren(
        h(
          'div',
          { class: 'text-center text-white bg-red-600/80 rounded-xl px-6 py-4 pointer-events-auto' },
          h('p', { class: 'font-semibold mb-1' }, t('captureFailed')),
          h('p', { class: 'text-sm opacity-90' }, e instanceof Error ? e.message : t('unknownError')),
        ),
      );
      setTimeout(backToLive, 2500);
    }
  };

  const collageShoot = async () => {
    if (busy) return;
    busy = true;
    bottomBar.classList.add('hidden');
    if (mode === 'on_demand') setLive(true);
    const n = COLLAGE_SHOTS[collageCfg.layout] ?? 4;
    const tokens: string[] = [];
    try {
      for (let s = 0; s < n; s++) {
        const cd = Math.max(0, Math.min(10, collageCfg.countdown_seconds));
        for (let i = cd; i >= 1; i--) {
          showCountdown(String(i), `Bild ${s + 1}/${n}`);
          beep();
          await sleep(800);
        }
        showCountdown('📸', `Bild ${s + 1}/${n}`);
        const photo = await api.capture();
        tokens.push(photo.token);
        showShotReview(photo, s + 1, n);
        await sleep(Math.max(0, collageCfg.review_seconds) * 1000);
      }
      showCountdown('✨');
      const collage = await api.composeCollage(tokens, collageCfg.layout);
      showResult(collage);
    } catch (e) {
      overlay.replaceChildren(
        h(
          'div',
          { class: 'text-center text-white bg-red-600/80 rounded-xl px-6 py-4 pointer-events-auto' },
          h('p', { class: 'font-semibold mb-1' }, t('captureFailed')),
          h('p', { class: 'text-sm opacity-90' }, e instanceof Error ? e.message : t('unknownError')),
        ),
      );
      setTimeout(backToLive, 2500);
    }
  };

  shutter.addEventListener('click', shoot);
  collageBtn.addEventListener('click', collageShoot);

  // --- Idle slideshow / screensaver ---
  let idleTimer: number | undefined;
  let slideTimer: number | undefined;
  let slideshowEl: HTMLElement | null = null;
  let comfort = { enabled: false, idle: 60, interval: 5 };

  const resetIdle = () => {
    clearTimeout(idleTimer);
    if (slideshowEl) return;
    if (comfort.enabled && comfort.idle > 0) {
      idleTimer = window.setTimeout(() => void startSlideshow(), comfort.idle * 1000);
    }
  };
  const stopSlideshow = () => {
    if (!slideshowEl) return;
    clearInterval(slideTimer);
    slideshowEl.remove();
    slideshowEl = null;
    resetIdle();
  };
  async function startSlideshow(): Promise<void> {
    if (slideshowEl || busy) {
      resetIdle();
      return;
    }
    let photos: Photo[] = [];
    try {
      photos = await api.photos(50, 0);
    } catch {
      /* ignore */
    }
    if (!photos.length) {
      resetIdle();
      return;
    }
    let i = 0;
    const img = h('img', {
      class: 'absolute inset-0 w-full h-full object-contain transition-opacity duration-700',
      src: api.photoUrl(photos[0]!.token),
    }) as HTMLImageElement;
    slideshowEl = h('div', { class: 'absolute inset-0 bg-black grid place-items-center z-30' }, img);
    stage.append(slideshowEl);
    slideTimer = window.setInterval(() => {
      i = (i + 1) % photos.length;
      img.style.opacity = '0';
      setTimeout(() => {
        img.src = api.photoUrl(photos[i]!.token);
        img.style.opacity = '1';
      }, 350);
    }, Math.max(2, comfort.interval) * 1000);
  }

  const onActivity = () => (slideshowEl ? stopSlideshow() : resetIdle());
  stage.addEventListener('pointerdown', onActivity);

  api
    .getComfort()
    .then((c) => {
      comfort = {
        enabled: c.slideshow_enabled,
        idle: c.slideshow_idle,
        interval: c.slideshow_interval,
      };
      resetIdle();
    })
    .catch(() => {});

  // Keyboard / hardware trigger: a key dismisses the slideshow, else shoots.
  const onKey = (ev: KeyboardEvent) => {
    if (slideshowEl) {
      ev.preventDefault();
      stopSlideshow();
      return;
    }
    resetIdle();
    if (ev.key === ' ' || ev.key === 'Enter') {
      ev.preventDefault();
      void shoot();
    }
  };
  window.addEventListener('keydown', onKey);

  // Detach listeners / timers when navigating away.
  window.addEventListener(
    'hashchange',
    () => {
      window.removeEventListener('keydown', onKey);
      clearTimeout(idleTimer);
      clearTimeout(resultTimer);
      clearInterval(slideTimer);
    },
    { once: true },
  );
}
