import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { VitePWA } from 'vite-plugin-pwa';

// The frontend builds straight into the Rust web crate's `assets` directory,
// where `rust-embed` picks it up (embedded in release, read from disk in debug).
export default defineConfig({
  plugins: [
    tailwindcss(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['icon.svg'],
      manifest: {
        name: 'SnapStation',
        short_name: 'SnapStation',
        description: 'Event-Foto-Station',
        lang: 'de',
        theme_color: '#0ea5e9',
        background_color: '#0b0f17',
        display: 'standalone',
        orientation: 'any',
        start_url: '/',
        icons: [
          { src: 'icon.svg', sizes: 'any', type: 'image/svg+xml', purpose: 'any' },
          { src: 'icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any' },
          { src: 'icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any maskable' },
        ],
      },
      workbox: {
        navigateFallback: '/index.html',
        navigateFallbackDenylist: [/^\/api/, /^\/p\//],
        runtimeCaching: [
          {
            // Photos carry a private share token. Prefer the network and keep
            // only a small, short-lived cache so deleted/revoked photos do not
            // linger on a shared kiosk device (privacy).
            urlPattern: ({ url }) => url.pathname.startsWith('/p/'),
            handler: 'NetworkFirst',
            options: {
              cacheName: 'snap-photos',
              networkTimeoutSeconds: 3,
              expiration: { maxEntries: 40, maxAgeSeconds: 60 * 30 },
            },
          },
        ],
      },
      devOptions: { enabled: false },
    }),
  ],
  // During `npm run dev`, proxy API + photo requests to the running Rust server.
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8099',
      '/p': 'http://127.0.0.1:8099',
    },
  },
  build: {
    outDir: '../crates/snap-web/assets',
    emptyOutDir: true,
  },
});
