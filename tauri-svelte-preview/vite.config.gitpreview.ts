/**
 * vite.config.gitpreview.ts — the ordinary dev server plus the git bridge,
 * on a port of its own.
 *
 * WHY THIS FILE. The source-control panes are being built as new files while
 * six other people work on the same checkout, so nothing shared may be edited —
 * and `vite.config.ts` is shared. This config leaves it exactly as it is and
 * adds the one plugin the browser needs to read a repository, so
 * `/git-preview` can be looked at without anyone else's dev server moving.
 *
 * Run it with:
 *     pnpm exec vite --config vite.config.gitpreview.ts
 * then open http://127.0.0.1:5187/git-preview
 *
 * SCRATCH. Once the integrator adds `gitBridgePlugin()` to `vite.config.ts`,
 * this file and the `/git-preview` route can both go.
 */
import { defineConfig, type Plugin, type UserConfig } from 'vite';

import baseConfig from './vite.config';
import { gitBridgePlugin } from './src/lib/server/gitBridge';

const base = baseConfig as UserConfig;

export default defineConfig({
  ...base,
  // Its own dependency cache: sharing the usual one would make every other dev
  // server in this checkout re-optimize its dependencies the moment this config
  // differs from theirs.
  cacheDir: 'node_modules/.vite-gitpreview',
  plugins: [gitBridgePlugin() as Plugin, ...((base.plugins ?? []) as Plugin[])],
  server: {
    ...(base.server ?? {}),
    port: 5187,
    strictPort: true
  }
});
