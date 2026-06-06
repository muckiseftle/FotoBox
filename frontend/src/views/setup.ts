import { api } from '../api';
import { h, clear, cls } from '../ui';

export async function renderSetup(app: HTMLElement): Promise<void> {
  clear(app);
  const status = await api.setupStatus().catch(() => null);

  const password = h('input', {
    type: 'password',
    class: cls.input,
    placeholder: 'Admin-Passwort (mindestens 6 Zeichen)',
    autocomplete: 'new-password',
  }) as HTMLInputElement;

  const eventName = h('input', {
    type: 'text',
    class: cls.input,
    placeholder: 'Event-Name (z. B. Hochzeit Anna & Tom)',
    value: status?.event_name ?? '',
  }) as HTMLInputElement;

  const language = h(
    'select',
    { class: cls.input },
    h('option', { value: 'de' }, 'Deutsch'),
    h('option', { value: 'en' }, 'English'),
  ) as HTMLSelectElement;
  language.value = status?.language ?? 'de';

  const error = h('p', { class: 'text-sm text-red-400 min-h-[1.25rem]' });

  const submit = h(
    'button',
    {
      class: cls.button + ' w-full',
      onclick: async () => {
        error.textContent = '';
        try {
          await api.submitSetup({
            admin_password: password.value,
            event_name: eventName.value,
            language: language.value,
          });
          // Log the operator in immediately so the admin area is reachable.
          await api.login(password.value).catch(() => {});
          location.hash = '#/';
        } catch (e) {
          error.textContent = e instanceof Error ? e.message : 'Fehler beim Einrichten.';
        }
      },
    },
    'Einrichten & loslegen',
  );

  app.append(
    h(
      'div',
      { class: 'min-h-full grid place-items-center px-4' },
      h(
        'div',
        { class: cls.card + ' w-full max-w-md' },
        h('h1', { class: 'text-2xl font-bold mb-1' }, 'Willkommen bei SnapStation'),
        h('p', { class: 'text-slate-400 mb-6 text-sm' }, 'Ersteinrichtung — dauert nur einen Moment.'),
        field('Admin-Passwort', password),
        field('Event-Name', eventName),
        field('Sprache', language),
        error,
        submit,
      ),
    ),
  );
}

function field(label: string, input: HTMLElement): HTMLElement {
  return h(
    'label',
    { class: 'block mb-4' },
    h('span', { class: 'block text-sm text-slate-300 mb-1' }, label),
    input,
  );
}
