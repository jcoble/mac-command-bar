import { dev } from '$app/environment';
import { error } from '@sveltejs/kit';

export function load(): Record<string, never> {
  if (!dev || !import.meta.env.DEV) {
    error(404, 'Development fixtures are only available in development.');
  }

  return {};
}
