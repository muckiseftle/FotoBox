import { api, type Photo } from '../api';
import { h, clear, cls, sleep } from '../ui';
import { theme } from '../theme';
import { t, getLang, setLang } from '../i18n';

// Fullscreen guest experience: live view, countdown, capture, result.
export function renderKiosk(app: HTMLElement): void {
  clear(app);

  const preview = h('img', {
    class: 'absolute inset-0 w-full h-full object-cover',
    alt: 'Live-Vorschau',
    src: api.previewStream(),
  }) as HTMLImageElement;

  const overlay = h('div', { class: 'absolute inset-0 grid place-items-center pointer-events-none' });

  const shutter = h(
    'button',
    {
      class:
        'absolute bottom-8 left-1/2 -translate-x-1/2 w-24 h-24 rounded-full bg-white/90 ' +
        'active:scale-95 transition-transform shadow-2xl ring-4 ring-brand/60',
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
      class: 'absolute top-4 right-4 text-slate-200/70 hover:text-white text-2xl',
      'aria-label': 'Admin',
    },
    '⚙',
  );

  const stage = h(
    'div',
    { class: 'kiosk relative w-full h-full bg-black overflow-hidden' },
    preview,
    overlay,
    brand,
    shutter,
    gear,
  );
  app.append(stage);

  // Guest-facing language toggle.
  const langToggle = h(
    'button',
    {
      class: 'absolute top-4 right-16 text-slate-200/70 hover:text-white text-sm font-semibold',
      'aria-label': 'Sprache / Language',
    },
    getLang().toUpperCase(),
  );
  langToggle.addEventListener('click', () => {
    setLang(getLang() === 'de' ? 'en' : 'de');
    renderKiosk(app);
  });
  stage.append(langToggle);

  let busy = false;

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
    })
    .catch(() => {});

  const showCountdown = (text: string) => {
    overlay.replaceChildren(
      h(
        'div',
        { class: 'text-white font-black drop-shadow-2xl text-[28vh] leading-none animate-pulse' },
        text,
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
            { class: 'flex gap-3' },
            printBtn,
            h('button', { class: cls.ghost, onclick: () => backToLive() }, t('retake')),
            h('a', { class: cls.ghost, href: '#/gallery' }, t('toGallery')),
          ),
          printStatus,
          h(
            'div',
            { class: 'flex items-center gap-3 text-slate-300 text-sm' },
            h('img', {
              class: 'w-24 h-24 rounded-lg bg-white p-1',
              src: api.qrUrl(photo.token),
              alt: 'QR-Code',
            }),
            h('span', {}, t('scanHint')),
          ),
        ),
      ),
    );
  };

  const backToLive = () => {
    overlay.replaceChildren();
    busy = false;
    preview.src = api.previewStream();
  };

  const shoot = async () => {
    if (busy) return;
    busy = true;
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

  shutter.addEventListener('click', shoot);

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
      clearInterval(slideTimer);
    },
    { once: true },
  );
}
