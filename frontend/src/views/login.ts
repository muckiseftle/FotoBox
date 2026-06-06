import { api } from '../api';
import { h, clear, cls } from '../ui';

/** Render the admin login form; calls `onSuccess` once authenticated. */
export function renderLogin(app: HTMLElement, onSuccess: () => void): void {
  clear(app);

  const password = h('input', {
    type: 'password',
    class: cls.input,
    placeholder: 'Admin-Passwort',
    autocomplete: 'current-password',
  }) as HTMLInputElement;
  const error = h('p', { class: 'text-sm text-red-400 min-h-[1.25rem]' });

  const submit = async () => {
    error.textContent = '';
    try {
      await api.login(password.value);
      onSuccess();
    } catch (e) {
      error.textContent = e instanceof Error ? e.message : 'Anmeldung fehlgeschlagen';
    }
  };
  password.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') void submit();
  });

  app.append(
    h(
      'div',
      { class: 'min-h-full grid place-items-center px-4' },
      h(
        'div',
        { class: cls.card + ' w-full max-w-sm' },
        h('h1', { class: 'text-xl font-bold mb-1' }, 'Admin-Anmeldung'),
        h('p', { class: 'text-slate-400 text-sm mb-4' }, 'Bitte mit dem Admin-Passwort anmelden.'),
        h('label', { class: 'block mb-3' }, password),
        error,
        h('button', { class: cls.button + ' w-full', onclick: () => void submit() }, 'Anmelden'),
        h('a', { class: 'block text-center text-sm text-slate-400 mt-4', href: '#/' }, 'Zur Kiosk-Ansicht'),
      ),
    ),
  );
}
