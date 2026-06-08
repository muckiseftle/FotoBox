import './style.css';
import { registerSW } from 'virtual:pwa-register';
import { api } from './api';
import { renderKiosk } from './views/kiosk';
import { renderGallery } from './views/gallery';
import { renderAdmin } from './views/admin';
import { renderSetup } from './views/setup';
import { renderChroma } from './views/chroma';
import { renderPrint } from './views/print';
import { renderDesign } from './views/design';
import { renderShare } from './views/teilen';
import { renderKioskSettings } from './views/kiosk-settings';
import { renderSettings } from './views/settings';
import { renderCamera } from './views/camera';
import { renderCapture } from './views/capture';
import { renderCollage } from './views/collage';
import { renderLogin } from './views/login';
import { applyTheme } from './theme';
import { initLang } from './i18n';

registerSW({ immediate: true });

const app = document.getElementById('app');
if (!app) throw new Error('#app missing');

async function route(): Promise<void> {
  void applyTheme(); // keep branding in sync (discards unsaved design tweaks)
  const hash = location.hash.replace(/^#/, '') || '/';

  // Force first-run setup until it has been completed.
  try {
    const setup = await api.setupStatus();
    if (!setup.complete && hash !== '/setup') {
      location.hash = '#/setup';
      return;
    }
  } catch {
    // Backend unreachable — fall through so the view can show an error.
  }

  // Admin routes require an authenticated session.
  const adminRoutes = ['/admin', '/camera', '/capture', '/collage', '/kiosk-settings', '/chroma', '/print', '/design', '/share', '/settings'];
  if (adminRoutes.includes(hash)) {
    let authed = false;
    try {
      authed = (await api.authStatus()).authenticated;
    } catch {
      authed = false;
    }
    if (!authed) {
      return renderLogin(app!, () => void route());
    }
  }

  switch (hash) {
    case '/gallery':
      return renderGallery(app!);
    case '/admin':
      return renderAdmin(app!);
    case '/camera':
      return renderCamera(app!);
    case '/capture':
      return renderCapture(app!);
    case '/collage':
      return renderCollage(app!);
    case '/kiosk-settings':
      return renderKioskSettings(app!);
    case '/chroma':
      return renderChroma(app!);
    case '/print':
      return renderPrint(app!);
    case '/design':
      return renderDesign(app!);
    case '/share':
      return renderShare(app!);
    case '/settings':
      return renderSettings(app!);
    case '/setup':
      return renderSetup(app!);
    default:
      return renderKiosk(app!);
  }
}

window.addEventListener('hashchange', () => void route());
void (async () => {
  await initLang();
  await route();
})();
