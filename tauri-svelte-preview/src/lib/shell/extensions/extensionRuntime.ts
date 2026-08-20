import {
  ExtensionHostKind,
  getBuiltinExtensions,
  registerExtension,
  type IExtensionManifest,
  type RegisterExtensionResult
} from '@codingame/monaco-vscode-api/extensions';

import './languageContributions';
import houstonThemeUrl from './houston/houston.json?url';
import svelteLanguageConfigurationUrl from './svelte/language-configuration.json?url';
import svelteStartTagConfigurationUrl from './svelte/language-configuration-start-tag.json?url';
import svelteGrammarUrl from './svelte/svelte.tmLanguage.json?url';
import svelteJavascriptSnippetsUrl from './svelte/snippets/javascript.json?url';
import svelteSnippetsUrl from './svelte/snippets/svelte.json?url';
import svelteTypescriptSnippetsUrl from './svelte/snippets/typescript.json?url';
import { enabledCuratedExtensions } from './extensionCatalog';
import mcbExtensionApiProbeBrowserUrl from '../editor/fixtures/mcbExtensionApiProbe.browser.cjs?url';
import {
  MCB_EXTENSION_API_PROBE_BROWSER_ENTRY,
  MCB_EXTENSION_API_PROBE_MANIFEST
} from '../editor/fixtures/mcbExtensionApiProbe';

type ExtensionRuntimeGlobal = typeof globalThis & {
  __mcbCuratedExtensionRegistrations?: Map<string, RegisterExtensionResult>;
};

const runtimeGlobal = globalThis as ExtensionRuntimeGlobal;
const registrations =
  runtimeGlobal.__mcbCuratedExtensionRegistrations ?? new Map<string, RegisterExtensionResult>();
runtimeGlobal.__mcbCuratedExtensionRegistrations = registrations;

function existingRegistration(id: string): RegisterExtensionResult {
  return {
    id,
    async dispose() {},
    async isEnabled() {
      return true;
    },
    async whenReady() {}
  };
}

function registrationAlreadyLoaded(id: string): boolean {
  return getBuiltinExtensions().some((extension) => extension.identifier.id === id);
}

/**
 * Register the curated declarative extensions exactly once.
 *
 * This runs before MonacoVscodeApiWrapper starts. registerExtension only adds
 * contributions to the compatibility layer; it does not initialize another
 * Monaco/VS Code service container or start another extension host.
 */
export function registerCuratedExtensions(): void {
  if (registrations.size > 0) return;

  for (const extension of enabledCuratedExtensions()) {
    if (registrationAlreadyLoaded(extension.id)) {
      registrations.set(extension.id, existingRegistration(extension.id));
      continue;
    }

    if (extension.id === 'mac-command-bar.extension-api-probe') {
      const registration = registerExtension(
        MCB_EXTENSION_API_PROBE_MANIFEST as unknown as IExtensionManifest,
        ExtensionHostKind.LocalWebWorker,
        { system: true }
      );
      registration.registerFileUrl(
        MCB_EXTENSION_API_PROBE_BROWSER_ENTRY,
        new URL(mcbExtensionApiProbeBrowserUrl, globalThis.location.href).href
      );
      registrations.set(extension.id, registration);
      continue;
    }

    if (extension.id === 'svelte.svelte-vscode-syntax') {
      const registration = registerExtension(
        {
          name: 'svelte-vscode-syntax',
          displayName: 'Svelte language basics',
          description: 'Official Svelte grammar, language metadata, and snippets.',
          categories: ['Programming Languages'],
          version: extension.version,
          publisher: 'svelte',
          engines: { vscode: '^1.82.0' },
          contributes: {
            languages: [
              {
                id: 'svelte',
                aliases: ['Svelte', 'svelte'],
                extensions: ['.svelte'],
                configuration: './language-configuration.json'
              },
              {
                id: 'svelte-start-tag',
                configuration: './language-configuration-start-tag.json'
              }
            ],
            grammars: [
              {
                language: 'svelte',
                scopeName: 'source.svelte',
                path: './syntaxes/svelte.tmLanguage.json',
                embeddedLanguages: {
                  'text.html.basic': 'html',
                  'text.html.markdown': 'markdown',
                  'meta.tag.start.svelte': 'svelte-start-tag',
                  'punctuation.definition.tag.begin.svelte': 'svelte',
                  'source.css': 'css',
                  'source.css.less': 'less',
                  'source.css.scss': 'scss',
                  'source.css.postcss': 'postcss',
                  'source.sass': 'sass',
                  'source.stylus': 'stylus',
                  'source.js': 'javascript',
                  'source.ts': 'typescript',
                  'source.coffee': 'coffeescript'
                },
                unbalancedBracketScopes: [
                  'keyword.operator.relational',
                  'storage.type.function.arrow',
                  'keyword.operator.bitwise.shift',
                  'meta.brace.angle',
                  'punctuation.definition.tag'
                ],
                tokenTypes: {
                  'punctuation.definition.template-expression': 'other',
                  'entity.name.type.instance.jsdoc': 'other',
                  'entity.name.function.tagged-template': 'other',
                  'meta.import string.quoted': 'other',
                  'variable.other.jsdoc': 'other'
                }
              }
            ],
            snippets: [
              { language: 'svelte', path: './snippets/svelte.json' },
              { language: 'javascript', path: './snippets/javascript.json' },
              { language: 'typescript', path: './snippets/typescript.json' }
            ]
          }
        },
        ExtensionHostKind.LocalWebWorker,
        { system: true }
      );
      registration.registerFileUrl('./language-configuration.json', svelteLanguageConfigurationUrl);
      registration.registerFileUrl(
        './language-configuration-start-tag.json',
        svelteStartTagConfigurationUrl
      );
      registration.registerFileUrl('./syntaxes/svelte.tmLanguage.json', svelteGrammarUrl);
      registration.registerFileUrl('./snippets/svelte.json', svelteSnippetsUrl);
      registration.registerFileUrl('./snippets/javascript.json', svelteJavascriptSnippetsUrl);
      registration.registerFileUrl('./snippets/typescript.json', svelteTypescriptSnippetsUrl);
      registrations.set(extension.id, registration);
      continue;
    }

    if (extension.id !== 'astro-build.houston') continue;

    const registration = registerExtension(
      {
        name: 'houston',
        displayName: 'Houston',
        description: 'The official Astro Houston color theme.',
        categories: ['Themes'],
        version: '0.1.0',
        publisher: 'astro-build',
        engines: { vscode: '^1.71.0' },
        contributes: {
          themes: [
            {
              id: 'Houston',
              label: 'Houston',
              uiTheme: 'vs-dark',
              path: './themes/houston.json'
            }
          ]
        }
      },
      ExtensionHostKind.LocalWebWorker,
      { system: true }
    );
    registration.registerFileUrl('./themes/houston.json', houstonThemeUrl);
    registrations.set(extension.id, registration);
  }
}

export function curatedExtensionIds(): string[] {
  return [...registrations.keys()];
}

export async function waitForCuratedExtensions(): Promise<void> {
  await Promise.all([...registrations.values()].map((registration) => registration.whenReady()));
}
