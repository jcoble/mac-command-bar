<script lang="ts">
	import type * as Monaco from "monaco-editor/esm/vs/editor/editor.api";
	import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
	import "monaco-editor/esm/vs/editor/contrib/gotoSymbol/browser/goToCommands";
	import "monaco-editor/esm/vs/editor/standalone/browser/referenceSearch/standaloneReferenceSearch";
	// Deep imports for the lazy target-model resolver (Task A2, design-monaco.md §4.2,
	// Route 1). The standalone `ITextModelService` is a global eager singleton
	// (standaloneServices.js: registerSingleton(ITextModelService, …, Eager)) and is
	// the exact instance the reference-peek tree resolves preview models through
	// (referencesWidget.js __param(4, ITextModelService) → DataSource →
	// FileReferences.resolve → createModelReference). Overriding its on-miss behaviour
	// makes peek read each file group lazily on expand instead of eagerly up front.
	import { StandaloneServices } from "monaco-editor/esm/vs/editor/standalone/browser/standaloneServices";
	import { ITextModelService } from "monaco-editor/esm/vs/editor/common/services/resolverService";
	import JsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
	import TypeScriptWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
	import "monaco-editor/min/vs/editor/editor.main.css";
	import { onDestroy, onMount } from "svelte";
	import {
		sourceCodeLensCountKey,
		sourceCodeLensCountMemoryMs,
		sourceCodeLensId,
		sourceCodeLensSpotFromId,
	} from "./sourceCodeLensKeys";
	import { sourcePreviewAppearance } from "./sourcePreviewAppearance";
	import { listThemes } from "$lib/shell/themes/themeRegistry";
	import { currentTheme, registerMonacoApplier } from "$lib/shell/themes/themeService";
	import { isNativeTauriRuntime } from "./tauriSource";
	import {
		extractSourceSemanticTokens,
		extractSourceSymbols,
		monacoLanguageForSource,
		sourceSemanticTokenLegend,
		type SourceCodeAction,
		type SourceCodeActionLookupRequest,
		type SourceCompletionItem,
		type SourceDiagnostic,
		type SourceDiagnosticSeverity,
		type SourceDefinitionTarget,
		type SourceDocumentHighlight,
		type SourceInlayHint,
		type SourcePreview,
		type SourceRecord,
		type SourceReferenceTarget,
		type SourceRenameResult,
		type SourceSemanticToken,
		type SourceSignatureHelp,
		type SourceSymbol,
		type SourceTextEdit,
	} from "./sourceData";

	type SourceEditorIntelligenceAction =
		| "completion"
		| "definition"
		| "hover"
		| "implementation"
		| "format"
		| "quick-fix"
		| "references"
		| "rename"
		| "signature-help"
		| "type-definition";

	type SourceEditorIntelligenceCommand = {
		id: number;
		action: SourceEditorIntelligenceAction;
	};

	type SourceEditorLookupRequest = {
		symbolName: string;
		line: number;
		column: number;
	};

	type SourceEditorProblemNavigationRequest = {
		line: number;
		column: number;
	};

	type SourceEditorProblemNavigation = (request: SourceEditorProblemNavigationRequest) => void;

	type SourceEditorHoverResult = {
		contents: string[];
	};

	type SourceEditorDefinitionLookup = (
		request: SourceEditorLookupRequest
	) => SourceDefinitionTarget[] | Promise<SourceDefinitionTarget[]> | null | undefined;

	type SourceEditorReferenceLookup = (
		request: SourceEditorLookupRequest
	) => SourceReferenceTarget[] | Promise<SourceReferenceTarget[]> | null | undefined;

	// A count may arrive as a bare number (older callers) or with `atLeast`,
	// which means the pass behind it did not finish reading the project, so the
	// real number can only be equal or higher.
	type SourceEditorReferenceCount = { count: number; atLeast: boolean };
	type SourceEditorReferenceCountLookup = (
		request: SourceEditorLookupRequest
	) =>
		| number
		| SourceEditorReferenceCount
		| Promise<number | SourceEditorReferenceCount | null>
		| null
		| undefined;

	type SourceEditorExternalPreviewLookup = (
		record: SourceRecord
	) => SourcePreview | Promise<SourcePreview | null> | null | undefined;

	type SourceEditorExternalNavigationRequest = {
		path: string;
		line: number;
		column: number;
	};

	type SourceEditorExternalNavigation = (
		request: SourceEditorExternalNavigationRequest
	) => void | Promise<void>;

	type SourceEditorImplementationLookup = (
		request: SourceEditorLookupRequest
	) => SourceDefinitionTarget[] | Promise<SourceDefinitionTarget[]> | null | undefined;

	type SourceEditorTypeDefinitionLookup = (
		request: SourceEditorLookupRequest
	) => SourceDefinitionTarget[] | Promise<SourceDefinitionTarget[]> | null | undefined;

	type SourceEditorCompletionLookup = (
		request: SourceEditorLookupRequest
	) => SourceCompletionItem[] | Promise<SourceCompletionItem[]> | null | undefined;

	type SourceEditorCodeActionLookup = (
		request: SourceCodeActionLookupRequest
	) => SourceCodeAction[] | Promise<SourceCodeAction[] | null> | null | undefined;

	type SourceEditorDocumentHighlightLookup = (
		request: SourceEditorLookupRequest
	) => SourceDocumentHighlight[] | Promise<SourceDocumentHighlight[] | null> | null | undefined;

	type SourceEditorSignatureHelpLookup = (
		request: SourceEditorLookupRequest
	) => SourceSignatureHelp | Promise<SourceSignatureHelp | null> | null | undefined;

	type SourceEditorInlayHintLookupRequest = {
		startLine: number;
		startColumn: number;
		endLine: number;
		endColumn: number;
	};

	type SourceEditorInlayHintLookup = (
		request: SourceEditorInlayHintLookupRequest
	) => SourceInlayHint[] | Promise<SourceInlayHint[] | null> | null | undefined;

	type SourceEditorSemanticTokensLookup = (
		preview: SourcePreview
	) =>
		| SourceSemanticToken[]
		| Promise<SourceSemanticToken[] | null>
		| null
		| undefined;

	type SourceEditorFormatDocument = () =>
		| SourceTextEdit[]
		| Promise<SourceTextEdit[]>
		| null
		| undefined;

	type SourceEditorRenameRequest = {
		line: number;
		column: number;
		newName: string;
	};

	type SourceEditorRename = (
		request: SourceEditorRenameRequest
	) => SourceRenameResult | Promise<SourceRenameResult | null> | null | undefined;

	type MonacoCommandService = {
		executeCommand: (id: string, ...args: unknown[]) => unknown;
	};

	type TypeScriptContribution = typeof import("monaco-editor/esm/vs/language/typescript/monaco.contribution");

	type Props = {
		preview: SourcePreview;
		content?: string;
		editable?: boolean;
		/**
		 * Optional appearance overrides sourced from user settings. When a field
		 * is present it is merged over the built-in `sourcePreviewAppearance`
		 * values in {@link applyAppearance}. When the whole prop is undefined or
		 * every field is undefined, behaviour is identical to the unconfigured
		 * default (uses `sourcePreviewAppearance` exactly).
		 */
		appearanceOverride?: {
			fontSize?: number;
			fontFamily?: string;
			lineHeight?: number;
		};
		externalDiagnostics?: SourceDiagnostic[];
		loading?: boolean;
		targetLine?: number | null;
		targetLineRequestId?: number;
		intelligenceCommand?: SourceEditorIntelligenceCommand | null;
		onContentChange?: (content: string) => void;
		onCodeActionLookup?: SourceEditorCodeActionLookup;
		onCommandPaletteRequest?: () => void;
		onCompletionLookup?: SourceEditorCompletionLookup;
		onDiagnosticsChange?: (diagnostics: SourceDiagnostic[]) => void;
		onDefinitionLookup?: SourceEditorDefinitionLookup;
		onDocumentHighlightLookup?: SourceEditorDocumentHighlightLookup;
		onExternalNavigation?: SourceEditorExternalNavigation;
		onExternalPreviewLookup?: SourceEditorExternalPreviewLookup;
		onFormatDocument?: SourceEditorFormatDocument;
		onGoToLineRequest?: () => void;
		onHoverLookup?: (request: SourceEditorLookupRequest) => SourceEditorHoverResult | Promise<SourceEditorHoverResult | null> | null;
		onImplementationLookup?: SourceEditorImplementationLookup;
		onNavigateBackRequest?: () => void;
		onNavigateForwardRequest?: () => void;
		onNextProblemRequest?: SourceEditorProblemNavigation;
		onPreviousProblemRequest?: SourceEditorProblemNavigation;
		onQuickOpenRequest?: () => void;
		onReferenceLookup?: SourceEditorReferenceLookup;
		onReferenceCountLookup?: SourceEditorReferenceCountLookup;
		onRename?: SourceEditorRename;
		onSaveRequest?: () => void;
		onInlayHintLookup?: SourceEditorInlayHintLookup;
		onSemanticTokensLookup?: SourceEditorSemanticTokensLookup;
		onSignatureHelpLookup?: SourceEditorSignatureHelpLookup;
		onSymbolsChange?: (symbols: SourceSymbol[]) => void;
		onTypeDefinitionLookup?: SourceEditorTypeDefinitionLookup;
		onWorkspaceEditAction?: (action: SourceCodeAction) => void | Promise<void>;
	};

	let {
		preview,
		content,
		editable = false,
		appearanceOverride,
		externalDiagnostics = [],
		loading = false,
		targetLine = null,
		targetLineRequestId = 0,
		intelligenceCommand = null,
		onContentChange,
		onCodeActionLookup,
		onCommandPaletteRequest,
		onCompletionLookup,
		onDiagnosticsChange,
		onDefinitionLookup,
		onDocumentHighlightLookup,
		onExternalNavigation,
		onExternalPreviewLookup,
		onFormatDocument,
		onGoToLineRequest,
		onHoverLookup,
		onImplementationLookup,
		onNavigateBackRequest,
		onNavigateForwardRequest,
		onNextProblemRequest,
		onPreviousProblemRequest,
		onQuickOpenRequest,
		onReferenceLookup,
		onReferenceCountLookup,
		onRename,
		onSaveRequest,
		onInlayHintLookup,
		onSemanticTokensLookup,
		onSignatureHelpLookup,
		onSymbolsChange,
		onTypeDefinitionLookup,
		onWorkspaceEditAction,
	}: Props = $props();

	let host = $state<HTMLDivElement | null>(null);
	let editor = $state<Monaco.editor.IStandaloneCodeEditor | null>(null);
	let monacoApi: typeof Monaco | null = null;
	let componentDestroyed = false;
	let contentChangeDisposable: Monaco.IDisposable | null = null;
	let markerChangeDisposable: Monaco.IDisposable | null = null;
	let modelChangeDisposable: Monaco.IDisposable | null = null;
	let mouseDefinitionDisposable: Monaco.IDisposable | null = null;
	let semanticTokensDisposable: Monaco.IDisposable | null = null;
	let hoverProviderDisposable: Monaco.IDisposable | null = null;
	let definitionProviderDisposable: Monaco.IDisposable | null = null;
	let documentHighlightProviderDisposable: Monaco.IDisposable | null = null;
	let implementationProviderDisposable: Monaco.IDisposable | null = null;
	let typeDefinitionProviderDisposable: Monaco.IDisposable | null = null;
	let formattingProviderDisposable: Monaco.IDisposable | null = null;
	let renameProviderDisposable: Monaco.IDisposable | null = null;
	let codeActionProviderDisposable: Monaco.IDisposable | null = null;
	let signatureHelpProviderDisposable: Monaco.IDisposable | null = null;
	let inlayHintsProviderDisposable: Monaco.IDisposable | null = null;
	let referenceProviderDisposable: Monaco.IDisposable | null = null;
	let codeLensProviderDisposable: Monaco.IDisposable | null = null;
	let completionProviderDisposable: Monaco.IDisposable | null = null;
	let documentSymbolProviderDisposable: Monaco.IDisposable | null = null;
	let editorActionDisposables: Monaco.IDisposable[] = [];
	let editorOpenerDisposable: Monaco.IDisposable | null = null;
	let currentPath = "";
	let currentTargetLine: number | null = null;
	let currentTargetLineRequestId = -1;
	let handledIntelligenceCommandId = -1;
	let applyingContent = false;
	let isReady = $state(false);
	// One short line in the corner of the editor, for work the user started that
	// takes long enough to look broken — today only the reference lookup behind
	// the "N references" margin numbers. Empty means nothing is shown.
	let editorNotice = $state("");
	let editorNoticeTimer = 0;
	let editorNoticeDelayTimer = 0;
	let layoutObserver: ResizeObserver | null = null;
	let layoutFrame = 0;
	let targetLineRevealFrame = 0;
	let externalWorkspaceEditCommandId = "";
	let codeLensReferenceCommandDisposable: Monaco.IDisposable | null = null;
	let monacoCancellationSuppressionDepth = 0;
	let monacoCancellationSuppressionTimer = 0;
	const ownedModels = new Set<Monaco.editor.ITextModel>();
	const inFlightTargetModels = new Map<string, Promise<Monaco.editor.ITextModel | null>>();
	// Bounded LRU of lazily-materialized EXTERNAL target models (design-monaco.md
	// §4.4). Most-recent path last. Only models created on-demand by the lazy
	// resolver / opener live here — never the currently-edited model. We cap growth
	// so a long find-refs session can't accumulate unbounded models, but we never
	// dispose on a benign model switch (re-opening a peek for the same symbol reuses
	// live models = zero re-reads). `onDestroy` still disposes everything in
	// `ownedModels`.
	const EXTERNAL_TARGET_MODEL_LRU_CAP = 24;
	const externalTargetModelLru = new Map<string, Monaco.editor.ITextModel>();
	// The standalone text-model resolver we shim for lazy materialization. Saved so
	// we can restore the original on destroy and delegate the wrap-as-reference work.
	type StandaloneTextModelResolver = {
		createModelReference: (
			resource: Monaco.Uri
		) => Promise<Monaco.editor.IReference<{ object: Monaco.editor.ITextModel }>>;
	};
	let textModelResolverService: StandaloneTextModelResolver | null = null;
	let originalCreateModelReference:
		| StandaloneTextModelResolver["createModelReference"]
		| null = null;
	// Numbers already given to us for the "N references" margin, kept for
	// `sourceCodeLensCountMemoryMs` so that editing the file does not re-ask for
	// every one of them. See `sourceCodeLensKeys.ts` for how they are named.
	const codeLensReferenceCountCache = new Map<
		string,
		{ count: number | null; atLeast: boolean; countedAt: number }
	>();
	const codeLensReferenceCommandId = "mcb.source.referenceCodeLens";
	// Hot-path deadline for the four navigation providers (design-monaco.md §2.3).
	// A nav lookup (definition/references/implementation/type-definition) must settle
	// within this budget with the best answer available now; a cold/slow/empty backend
	// returns fast (empty peek that can re-populate) instead of freezing the gesture.
	const NAV_LOOKUP_DEADLINE_MS = 200;
	/**
	 * The theme in force, kept here as state so the markup below follows a switch.
	 * `currentTheme()` is a plain function, not a rune, so reading it in a
	 * `$derived` would freeze at whatever was in force when this editor was
	 * created — the applier registered below is what keeps this honest.
	 */
	let activeTheme = $state(currentTheme());
	/**
	 * Repaint this editor when the theme changes. Called once immediately with the
	 * theme in force, and again on every switch.
	 *
	 * `setTheme` is only safe once Monaco has loaded, because `configureMonaco` is
	 * where every theme is defined; before that, remembering which theme is in
	 * force is enough, and the editor is created with it.
	 */
	const disposeThemeApplier = registerMonacoApplier((theme) => {
		activeTheme = theme;
		monacoApi?.editor.setTheme(theme.monaco.id);
	});
	/** The colour behind the editor before Monaco has painted a frame. It follows
	 * the theme, or a Dracula editor flashes Houston's background on the way in. */
	const editorBackground = $derived(
		activeTheme.monaco.colors["editor.background"] ?? "#17191e",
	);
	const sourceLspMonacoLanguageIDs = [
		"typescript",
		"javascript",
		"csharp",
		"rust",
		"html",
	];
	const sourceCodeLensSymbolKinds = new Set([
		"class",
		"constructor",
		"enum",
		"function",
		"interface",
		"method",
		"property",
		"record",
		"struct",
		"variable",
	]);

	function installWorker() {
		const target = self as unknown as {
			MonacoEnvironment?: { getWorker: (_moduleId: string, _label: string) => Worker };
		};

		target.MonacoEnvironment = {
			getWorker: (_moduleId, label) => {
				if (label === "json") return new JsonWorker();
				if (label === "typescript" || label === "javascript") return new TypeScriptWorker();
				return new EditorWorker();
			},
		};
	}

	/**
	 * Define every theme the shell ships, and hand Monaco's repainter to the theme
	 * service so a switch reaches the editor. Runs once per Monaco load, which is
	 * the right scope: `setTheme` is global to Monaco, so one registration covers
	 * however many editors exist.
	 */
	function configureMonaco(monaco: typeof Monaco) {
		for (const theme of listThemes()) {
			const { id, ...definition } = theme.monaco;
			monaco.editor.defineTheme(id, {
				...definition,
				rules: definition.rules ?? [],
				colors: definition.colors ?? {},
			});
		}
	}

	function configureTypeScriptLanguageService(typeScriptLanguage: TypeScriptContribution) {
		const compilerOptions = {
			allowNonTsExtensions: true,
			allowJs: true,
			checkJs: false,
			module: typeScriptLanguage.ModuleKind.ESNext,
			moduleResolution: typeScriptLanguage.ModuleResolutionKind.NodeJs,
			noEmit: true,
			strict: true,
			target: typeScriptLanguage.ScriptTarget.ESNext,
		};
		// When the Rust LSP backend is available (native Tauri), it owns TS/JS
		// diagnostics — they arrive as `mcb-lsp` markers via `externalDiagnostics`.
		// Suppress the bundled worker's diagnostics so it doesn't double up. In the
		// web preview / offline (no Tauri → no Rust path), keep the worker's
		// diagnostics so squiggles still appear. The worker stays loaded either way,
		// so completions/folding/outline/quick-info remain available.
		const suppressWorkerDiagnostics = isNativeTauriRuntime();
		const diagnosticsOptions = {
			noSemanticValidation: suppressWorkerDiagnostics,
			noSyntaxValidation: suppressWorkerDiagnostics,
			noSuggestionDiagnostics: suppressWorkerDiagnostics,
		};

		typeScriptLanguage.typescriptDefaults.setEagerModelSync(true);
		typeScriptLanguage.typescriptDefaults.setCompilerOptions(compilerOptions);
		typeScriptLanguage.typescriptDefaults.setDiagnosticsOptions(diagnosticsOptions);
		typeScriptLanguage.javascriptDefaults.setEagerModelSync(true);
		typeScriptLanguage.javascriptDefaults.setCompilerOptions(compilerOptions);
		typeScriptLanguage.javascriptDefaults.setDiagnosticsOptions(diagnosticsOptions);
	}

	function registerSourceSemanticTokens(monaco: typeof Monaco) {
		semanticTokensDisposable?.dispose();
		semanticTokensDisposable = monaco.languages.registerDocumentSemanticTokensProvider(
			sourceLspMonacoLanguageIDs,
			{
				getLegend: () => ({
					tokenTypes: [...sourceSemanticTokenLegend.tokenTypes],
					tokenModifiers: [...sourceSemanticTokenLegend.tokenModifiers],
				}),
				provideDocumentSemanticTokens: async (model) => {
					const modelPreview = previewForModel(model);
					const nativeTokens = (await onSemanticTokensLookup?.(modelPreview)) ?? [];
					const tokens =
						nativeTokens.length > 0
							? nativeTokens
							: extractSourceSemanticTokens(modelPreview, model.getValue());
					return { data: encodeSemanticTokens(tokens) };
				},
				releaseDocumentSemanticTokens: () => {},
			}
		);
	}

	function registerSourceHoverProvider(monaco: typeof Monaco) {
		hoverProviderDisposable?.dispose();
		hoverProviderDisposable = monaco.languages.registerHoverProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideHover: async (model, position) => {
					const word = model.getWordAtPosition(position);
					if (!word) return null;

					const sourcePreview = previewForModel(model);
					const range = new monaco.Range(
						position.lineNumber,
						word.startColumn,
						position.lineNumber,
						word.endColumn
					);
					const lspHover = await onHoverLookup?.({
						symbolName: word.word,
						line: position.lineNumber,
						column: word.startColumn,
					});
					if (lspHover?.contents.length) {
						return {
							range,
							contents: lspHover.contents.map((value) => ({ value })),
						};
					}

					const symbol = extractSourceSymbols(sourcePreview, model.getValue()).find(
						(candidate) => candidate.name === word.word
					);
					if (!symbol) return null;

					return {
						range,
						contents: [
							{ value: `**${symbol.kind}** \`${symbol.name}\`` },
							{ value: `\`\`\`${sourcePreview.language}\n${symbol.detail}\n\`\`\`` },
						],
					};
				},
			}
		);
	}

	// Race a hot-path lookup against a fixed deadline AND Monaco's CancellationToken
	// (design-monaco.md §2.3). Resolves to the real value when the promise wins first;
	// resolves to `null` when the deadline elapses, when the token is cancelled, or when
	// the underlying promise rejects (a slow/flaky backend must never throw or hang the
	// gesture). The sleep timer and the cancellation listener are always torn down once a
	// racer wins, so neither a timer nor an event subscription leaks.
	function withDeadline<T>(
		p: Promise<T>,
		ms: number,
		token: Monaco.CancellationToken
	): Promise<T | null> {
		return new Promise<T | null>((resolve) => {
			let settled = false;
			let timer = 0;
			let cancelSubscription: Monaco.IDisposable | null = null;

			const finish = (value: T | null) => {
				if (settled) return;
				settled = true;
				if (timer) {
					window.clearTimeout(timer);
					timer = 0;
				}
				cancelSubscription?.dispose();
				cancelSubscription = null;
				resolve(value);
			};

			// Already-cancelled tokens short-circuit immediately (no work, no timer).
			if (token.isCancellationRequested) {
				finish(null);
				return;
			}

			timer = window.setTimeout(() => finish(null), ms);
			cancelSubscription = token.onCancellationRequested(() => finish(null));
			p.then(
				(value) => finish(value),
				() => finish(null)
			);
		});
	}

	function registerSourceDefinitionProvider(monaco: typeof Monaco) {
		definitionProviderDisposable?.dispose();
		definitionProviderDisposable = monaco.languages.registerDefinitionProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideDefinition: async (model, position, token) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return null;

					// A3: never block the gesture — race the lookup against a short deadline
					// and Monaco's cancellation token. Cancelled ⇒ null (Monaco discards, so a
					// superseded request can't paint a stale jump/peek); deadline/empty ⇒ [].
					const result = await withDeadline(
						Promise.resolve(onDefinitionLookup?.(request)),
						NAV_LOOKUP_DEADLINE_MS,
						token
					);
					if (token.isCancellationRequested) return null;
					if (!result) return [];
					// A2: return locations only — no eager fan-out read. Target models are
					// materialized lazily (peek group on expand via the resolver shim, or the
					// single navigated model in the editor opener on a jump).
					return result.map((target) => sourceDefinitionTargetToLocation(monaco, target));
				},
			}
		);
	}

	function registerSourceReferenceProvider(monaco: typeof Monaco) {
		referenceProviderDisposable?.dispose();
		referenceProviderDisposable = monaco.languages.registerReferenceProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideReferences: async (model, position, _context, token) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return null;

					// A3: never block the gesture — race the lookup against a short deadline
					// and Monaco's cancellation token. Cancelled ⇒ null (so a rapid re-trigger
					// can't paint a stale peek); deadline/empty ⇒ [] (an empty peek that can
					// re-populate beats a 30s spinner hang).
					const result = await withDeadline(
						Promise.resolve(onReferenceLookup?.(request)),
						NAV_LOOKUP_DEADLINE_MS,
						token
					);
					if (token.isCancellationRequested) return null;
					if (!result) return [];
					// A2: locations only — Monaco reads each file group lazily on expand via
					// the lazy text-model resolver (createModelReference shim). No read storm.
					return result.map((target) => sourceReferenceTargetToLocation(monaco, target));
				},
			}
		);
	}

	function registerSourceCodeLensProvider(monaco: typeof Monaco) {
		codeLensProviderDisposable?.dispose();
		codeLensProviderDisposable = monaco.languages.registerCodeLensProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideCodeLenses: (model) => {
					const modelPreview = previewForModel(model);
					const symbols = extractSourceSymbols(modelPreview, model.getValue())
						.filter(isSourceCodeLensSymbol)
						.slice(0, 120);

					return {
						lenses: symbols.map((symbol) => {
							const request = sourceSymbolToLookupRequest(symbol);
							return {
								id: sourceCodeLensId(request),
								range: new monaco.Range(symbol.line, 1, symbol.line, 1),
							};
						}),
						dispose() {},
					};
				},
				resolveCodeLens: async (model, codeLens, token) => {
					const request = sourceCodeLensLookupRequest(codeLens);
					if (!request || token.isCancellationRequested) return codeLens;

					const counted = await sourceCodeLensReferenceCount(model, request);
					if (token.isCancellationRequested) return codeLens;
					// Unknown count (no LSP / skipped large-repo scan) ⇒ leave the lens
					// without a command so Monaco shows no "N references" instead of "0".
					if (counted === null) return codeLens;

					return {
						...codeLens,
						command: {
							id: codeLensReferenceCommandId,
							title: formatReferenceCodeLensTitle(counted.count, counted.atLeast),
							arguments: [request],
						},
					};
				},
			}
		);
	}

	function registerSourceDocumentHighlightProvider(monaco: typeof Monaco) {
		documentHighlightProviderDisposable?.dispose();
		documentHighlightProviderDisposable = monaco.languages.registerDocumentHighlightProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideDocumentHighlights: async (model, position) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return [];

					const highlights = await onDocumentHighlightLookup?.(request);
					return (highlights ?? []).map((highlight) =>
						sourceDocumentHighlightToMonacoHighlight(monaco, highlight)
					);
				},
			}
		);
	}

	function registerSourceImplementationProvider(monaco: typeof Monaco) {
		implementationProviderDisposable?.dispose();
		implementationProviderDisposable = monaco.languages.registerImplementationProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideImplementation: async (model, position, token) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return null;

					// A3: never block the gesture — race the lookup against a short deadline
					// and Monaco's cancellation token. Cancelled ⇒ null; deadline/empty ⇒ [].
					const result = await withDeadline(
						Promise.resolve(onImplementationLookup?.(request)),
						NAV_LOOKUP_DEADLINE_MS,
						token
					);
					if (token.isCancellationRequested) return null;
					if (!result) return [];
					// A2: locations only; lazy resolver materializes peek previews on expand.
					return result.map((target) =>
						sourceImplementationTargetToLocation(monaco, target)
					);
				},
			}
		);
	}

	function registerSourceTypeDefinitionProvider(monaco: typeof Monaco) {
		typeDefinitionProviderDisposable?.dispose();
		typeDefinitionProviderDisposable = monaco.languages.registerTypeDefinitionProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideTypeDefinition: async (model, position, token) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return null;

					// A3: never block the gesture — race the lookup against a short deadline
					// and Monaco's cancellation token. Cancelled ⇒ null; deadline/empty ⇒ [].
					const result = await withDeadline(
						Promise.resolve(onTypeDefinitionLookup?.(request)),
						NAV_LOOKUP_DEADLINE_MS,
						token
					);
					if (token.isCancellationRequested) return null;
					if (!result) return [];
					// A2: locations only; lazy resolver materializes peek previews on expand.
					return result.map((target) =>
						sourceTypeDefinitionTargetToLocation(monaco, target)
					);
				},
			}
		);
	}

	function registerSourceFormattingProvider(monaco: typeof Monaco) {
		formattingProviderDisposable?.dispose();
		formattingProviderDisposable = monaco.languages.registerDocumentFormattingEditProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideDocumentFormattingEdits: async () => {
					const edits = await onFormatDocument?.();
					return (edits ?? []).map((edit) => sourceTextEditToMonacoEdit(monaco, edit));
				},
			}
		);
	}

	function registerSourceRenameProvider(monaco: typeof Monaco) {
		renameProviderDisposable?.dispose();
		renameProviderDisposable = monaco.languages.registerRenameProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideRenameEdits: async (model, position, newName) => {
					const normalizedName = newName.trim();
					if (!normalizedName) return { edits: [] };

					const result = await onRename?.({
						line: position.lineNumber,
						column: position.column,
						newName: normalizedName,
					});
					const currentFileEdits =
						(result?.files ?? []).find((file) => file.path === currentPath)?.edits ?? [];

					return {
						edits: currentFileEdits.map((edit) => ({
							resource: model.uri,
							textEdit: sourceTextEditToMonacoEdit(monaco, edit),
							versionId: model.getVersionId(),
						})),
					};
				},
			}
		);
	}

	function registerSourceCodeActionProvider(monaco: typeof Monaco) {
		codeActionProviderDisposable?.dispose();
		codeActionProviderDisposable = monaco.languages.registerCodeActionProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideCodeActions: async (_model, range, context) => {
					const actions = await onCodeActionLookup?.({
						startLine: range.startLineNumber,
						startColumn: range.startColumn,
						endLine: range.endLineNumber,
						endColumn: range.endColumn,
						diagnostics: context.markers.map((marker) => ({
							severity: markerSeverityToSourceSeverity(marker.severity),
							message: marker.message,
							startLine: marker.startLineNumber,
							startColumn: marker.startColumn,
							endLine: marker.endLineNumber,
							endColumn: marker.endColumn,
							source: marker.source ?? undefined,
						})),
					});

					return {
						actions: (actions ?? []).map((action) => sourceCodeActionToMonacoAction(monaco, action)),
						dispose() {},
					};
				},
			},
			{
				providedCodeActionKinds: ["quickfix", "refactor", "source"],
			}
		);
	}

	function registerSourceSignatureHelpProvider(monaco: typeof Monaco) {
		signatureHelpProviderDisposable?.dispose();
		signatureHelpProviderDisposable = monaco.languages.registerSignatureHelpProvider(
			sourceLspMonacoLanguageIDs,
			{
				signatureHelpTriggerCharacters: ["(", ",", "<"],
				signatureHelpRetriggerCharacters: [","],
				provideSignatureHelp: async (model, position) => {
					const request = positionRequestForModelPosition(model, position);
					const signatureHelp = await onSignatureHelpLookup?.(request);
					if (!signatureHelp?.signatures.length) return null;

					return {
						value: sourceSignatureHelpToMonacoHelp(signatureHelp),
						dispose() {},
					};
				},
			}
		);
	}

	function registerSourceInlayHintsProvider(monaco: typeof Monaco) {
		inlayHintsProviderDisposable?.dispose();
		inlayHintsProviderDisposable = monaco.languages.registerInlayHintsProvider(
			sourceLspMonacoLanguageIDs,
			{
				displayName: "MacCommandBar LSP",
				provideInlayHints: async (_model, range) => {
					const hints = await onInlayHintLookup?.({
						startLine: range.startLineNumber,
						startColumn: range.startColumn,
						endLine: range.endLineNumber,
						endColumn: range.endColumn,
					});

					return {
						hints: (hints ?? []).map((hint) => sourceInlayHintToMonacoHint(monaco, hint)),
						dispose() {},
					};
				},
			}
		);
	}

	function registerSourceCompletionProvider(monaco: typeof Monaco) {
		completionProviderDisposable?.dispose();
		completionProviderDisposable = monaco.languages.registerCompletionItemProvider(
			sourceLspMonacoLanguageIDs,
			{
				triggerCharacters: [".", ":", "<", '"', "'", "/"],
				provideCompletionItems: async (model, position) => {
					const request = completionRequestForModelPosition(model, position);
					const completions = await onCompletionLookup?.(request);
					const word = model.getWordUntilPosition(position);
					const range = new monaco.Range(
						position.lineNumber,
						word.startColumn,
						position.lineNumber,
						word.endColumn
					);

					return {
						suggestions: (completions ?? []).map((item) =>
							sourceCompletionItemToSuggestion(monaco, item, range)
						),
					};
				},
			}
		);
	}

	function registerSourceDocumentSymbolProvider(monaco: typeof Monaco) {
		documentSymbolProviderDisposable?.dispose();
		documentSymbolProviderDisposable = monaco.languages.registerDocumentSymbolProvider(
			sourceLspMonacoLanguageIDs,
			{
				provideDocumentSymbols: (model) =>
					extractSourceSymbols(previewForModel(model), model.getValue()).map((symbol) =>
						sourceSymbolToDocumentSymbol(monaco, symbol)
					),
			}
		);
	}

	function sourcePathFromMonacoUri(uri: Monaco.Uri | null | undefined) {
		if (!uri) return "";
		return uri.fsPath || decodeURIComponent(uri.path || "");
	}

	function locationFromMonacoSelection(
		selection: Monaco.IRange | Monaco.IPosition | null | undefined
	) {
		const value = selection as
			| Partial<
					Monaco.IRange &
						Monaco.IPosition & {
							selectionStartLineNumber: number;
							selectionStartColumn: number;
							positionLineNumber: number;
							positionColumn: number;
						}
			  >
			| null
			| undefined;
		const line =
			value?.startLineNumber ??
			value?.selectionStartLineNumber ??
			value?.lineNumber ??
			value?.positionLineNumber ??
			1;
		const column =
			value?.startColumn ??
			value?.selectionStartColumn ??
			value?.column ??
			value?.positionColumn ??
			1;
		return { line: Math.max(1, line), column: Math.max(1, column) };
	}

	// Mark an external target model as most-recently-used and evict beyond the cap
	// (design-monaco.md §4.4). Never evicts the model the editor is currently showing
	// (the currently-edited model is never an external target, but a peeked target can
	// momentarily be the active model during a jump — guard anyway). Evicted models are
	// disposed and dropped from `ownedModels`.
	function touchExternalTargetModel(path: string, model: Monaco.editor.ITextModel) {
		// Re-insert to move to the most-recent (last) position.
		externalTargetModelLru.delete(path);
		externalTargetModelLru.set(path, model);
		if (externalTargetModelLru.size <= EXTERNAL_TARGET_MODEL_LRU_CAP) return;

		const activeUri = editor?.getModel()?.uri.toString();
		for (const [lruPath, lruModel] of externalTargetModelLru) {
			if (externalTargetModelLru.size <= EXTERNAL_TARGET_MODEL_LRU_CAP) break;
			// Skip the live/active model and any already-disposed entry's owner.
			if (lruModel.uri.toString() === activeUri) continue;
			externalTargetModelLru.delete(lruPath);
			if (!lruModel.isDisposed()) {
				ownedModels.delete(lruModel);
				lruModel.dispose();
			}
		}
	}

	// On-miss body for the lazy resolver (Task A2): create exactly one external target
	// model on demand from A1's memoized `onExternalPreviewLookup`. Kept single (NOT a
	// bulk fan-out) and de-duped in-flight so def+ref providers firing together share
	// one read. This is invoked lazily — per navigated jump target and per expanded
	// peek file group — never eagerly across a whole result set.
	async function ensureSourceTargetModel(monaco: typeof Monaco, target: SourceRecord) {
		const uri = monaco.Uri.file(target.path);
		const existing = monaco.editor.getModel(uri);
		if (existing) {
			if (ownedModels.has(existing)) touchExternalTargetModel(target.path, existing);
			return existing;
		}

		// Dedupe concurrent requests for the same path so two callers (e.g. the peek
		// resolver and a parallel jump) don't each read the same file before any
		// createModel() lands — the duplicate read_source_file reads.
		const pending = inFlightTargetModels.get(target.path);
		if (pending) return pending;

		const load = (async () => {
			const externalPreview = await onExternalPreviewLookup?.(target);
			if (!externalPreview) return null;
			// A peer request may have created the model while we awaited the read.
			const raced = monaco.editor.getModel(uri);
			if (raced) {
				if (ownedModels.has(raced)) touchExternalTargetModel(target.path, raced);
				return raced;
			}
			const model = monaco.editor.createModel(
				externalPreview.content,
				monacoLanguageForSource(externalPreview.language),
				uri
			);
			ownedModels.add(model);
			touchExternalTargetModel(target.path, model);
			return model;
		})().finally(() => {
			inFlightTargetModels.delete(target.path);
		});

		inFlightTargetModels.set(target.path, load);
		return load;
	}

	// Route 1 (design-monaco.md §4.2): override the standalone text-model resolver so
	// the reference-peek tree materializes a file's model lazily, on expand, instead of
	// rejecting with "Model not found" (standaloneServices.js:128). On a miss we read
	// just that one file via `ensureSourceTargetModel`, then delegate to the original
	// `createModelReference`, which now finds the model and wraps it as the immortal
	// reference Monaco expects. Returning a `Location[]` from a provider therefore reads
	// ZERO files; only an expanded peek group (or a followed jump) triggers a read.
	function installLazyTargetModelResolver(monaco: typeof Monaco) {
		if (textModelResolverService) return;
		let service: StandaloneTextModelResolver | null = null;
		try {
			service = StandaloneServices.get(
				ITextModelService
			) as unknown as StandaloneTextModelResolver;
		} catch (error) {
			console.warn("Lazy target-model resolver unavailable; peek previews may be empty", error);
			return;
		}
		if (!service || typeof service.createModelReference !== "function") return;
		if (originalCreateModelReference) return; // already shimmed

		textModelResolverService = service;
		originalCreateModelReference = service.createModelReference.bind(service);
		const delegate = originalCreateModelReference;

		service.createModelReference = async (resource: Monaco.Uri) => {
			// Fast path: model already exists (currently-edited file, a prior peek
			// target, or any model Monaco created). Delegate straight through; keep the
			// LRU warm so reusing it survives eviction pressure.
			if (monaco.editor.getModel(resource)) {
				const path = sourcePathFromMonacoUri(resource);
				const known = path ? externalTargetModelLru.get(path) : undefined;
				if (path && known) touchExternalTargetModel(path, known);
				return delegate(resource);
			}

			const path = sourcePathFromMonacoUri(resource);
			if (path) {
				// `onExternalPreviewLookup` (page side) only reads `record.path`; it
				// reconstructs the full SourceRecord from the project itself. So a
				// path-only record is sufficient and correct here.
				await ensureSourceTargetModel(monaco, { path } as SourceRecord);
			}
			// Delegate regardless: if we materialized the model the original now wraps
			// it; if we couldn't (no preview), the original rejects exactly as before and
			// FileReferences.resolve swallows it per-child (empty preview, not a crash).
			return delegate(resource);
		};
	}

	function uninstallLazyTargetModelResolver() {
		if (textModelResolverService && originalCreateModelReference) {
			textModelResolverService.createModelReference = originalCreateModelReference;
		}
		textModelResolverService = null;
		originalCreateModelReference = null;
	}

	function sourceDefinitionTargetToLocation(
		monaco: typeof Monaco,
		target: SourceDefinitionTarget
	): Monaco.languages.Location {
		const line = Math.max(1, target.line);
		const column = Math.max(1, target.column);
		const length = Math.max(1, target.symbolName.length);
		return {
			uri: monaco.Uri.file(target.path),
			range: new monaco.Range(line, column, line, column + length),
		};
	}

	function sourceImplementationTargetToLocation(
		monaco: typeof Monaco,
		target: SourceDefinitionTarget
	): Monaco.languages.Location {
		return sourceDefinitionTargetToLocation(monaco, target);
	}

	function sourceTypeDefinitionTargetToLocation(
		monaco: typeof Monaco,
		target: SourceDefinitionTarget
	): Monaco.languages.Location {
		return sourceDefinitionTargetToLocation(monaco, target);
	}

	function sourceTextEditToMonacoEdit(
		monaco: typeof Monaco,
		edit: SourceTextEdit
	): Monaco.languages.TextEdit {
		return {
			range: new monaco.Range(
				Math.max(1, edit.startLine),
				Math.max(1, edit.startColumn),
				Math.max(1, edit.endLine),
				Math.max(1, edit.endColumn)
			),
			text: edit.newText,
		};
	}

	function sourceCodeActionToMonacoAction(
		monaco: typeof Monaco,
		action: SourceCodeAction
	): Monaco.languages.CodeAction {
		const model = editor?.getModel();
		const currentFile = action.files.find((file) => file.path === currentPath);
		const currentFileEdits = currentFile?.edits ?? [];
		const externalFiles = action.files.filter((file) => file.path !== currentPath && file.edits.length > 0);
		const edit =
			model && currentFileEdits.length > 0
				? {
						edits: currentFileEdits.map((textEdit) => ({
							resource: model.uri,
							textEdit: sourceTextEditToMonacoEdit(monaco, textEdit),
							versionId: model.getVersionId(),
						})),
					}
				: undefined;
		const command =
			externalFiles.length > 0 && externalWorkspaceEditCommandId
				? {
						id: externalWorkspaceEditCommandId,
						title: "Stage External Workspace Edits",
						arguments: [action],
					}
				: undefined;
		const disabled =
			action.disabledReason ??
			(edit || command ? undefined : "No current-file edit available");

		return {
			title: action.title,
			kind: action.kind || "quickfix",
			isPreferred: action.isPreferred,
			disabled,
			edit,
			command,
		};
	}

	function sourceSignatureHelpToMonacoHelp(
		signatureHelp: SourceSignatureHelp
	): Monaco.languages.SignatureHelp {
		return {
			activeSignature: Math.max(0, signatureHelp.activeSignature),
			activeParameter: Math.max(0, signatureHelp.activeParameter),
			signatures: signatureHelp.signatures.map((signature) => ({
				label: signature.label,
				documentation: signature.documentation ? { value: signature.documentation } : undefined,
				parameters: signature.parameters.map((parameter) => ({
					label: parameter.label,
					documentation: parameter.documentation ? { value: parameter.documentation } : undefined,
				})),
			})),
		};
	}

	function sourceInlayHintToMonacoHint(
		monaco: typeof Monaco,
		hint: SourceInlayHint
	): Monaco.languages.InlayHint {
		const kind =
			hint.kind === "parameter"
				? monaco.languages.InlayHintKind.Parameter
				: hint.kind === "type"
					? monaco.languages.InlayHintKind.Type
					: undefined;

		return {
			label: hint.label,
			tooltip: hint.tooltip ? { value: hint.tooltip } : undefined,
			kind,
			position: {
				lineNumber: Math.max(1, hint.line),
				column: Math.max(1, hint.column),
			},
			paddingLeft: hint.paddingLeft,
			paddingRight: hint.paddingRight,
		};
	}

	function sourceReferenceTargetToLocation(
		monaco: typeof Monaco,
		target: SourceReferenceTarget
	): Monaco.languages.Location {
		const line = Math.max(1, target.line);
		const column = Math.max(1, target.column);
		const length = Math.max(1, target.symbolName.length);
		return {
			uri: monaco.Uri.file(target.path),
			range: new monaco.Range(line, column, line, column + length),
		};
	}

	function sourceDocumentHighlightToMonacoHighlight(
		monaco: typeof Monaco,
		highlight: SourceDocumentHighlight
	): Monaco.languages.DocumentHighlight {
		const kind = monaco.languages.DocumentHighlightKind;
		const highlightKind =
			highlight.kind === "read"
				? kind.Read
				: highlight.kind === "write"
					? kind.Write
					: kind.Text;

		return {
			range: new monaco.Range(
				Math.max(1, highlight.startLine),
				Math.max(1, highlight.startColumn),
				Math.max(1, highlight.endLine),
				Math.max(1, highlight.endColumn)
			),
			kind: highlightKind,
		};
	}

	function sourceCompletionItemToSuggestion(
		monaco: typeof Monaco,
		item: SourceCompletionItem,
		range: Monaco.IRange
	): Monaco.languages.CompletionItem {
		return {
			label: item.label,
			kind: sourceCompletionItemKind(monaco, item.kind),
			detail: item.detail || undefined,
			insertText: item.insertText || item.label,
			range,
		};
	}

	function sourceCompletionItemKind(
		monaco: typeof Monaco,
		kind: string
	): Monaco.languages.CompletionItemKind {
		const completionKind = monaco.languages.CompletionItemKind;
		switch (kind) {
			case "method":
				return completionKind.Method;
			case "function":
				return completionKind.Function;
			case "constructor":
				return completionKind.Constructor;
			case "field":
				return completionKind.Field;
			case "variable":
				return completionKind.Variable;
			case "class":
				return completionKind.Class;
			case "interface":
				return completionKind.Interface;
			case "module":
				return completionKind.Module;
			case "property":
				return completionKind.Property;
			case "unit":
				return completionKind.Unit;
			case "value":
				return completionKind.Value;
			case "enum":
				return completionKind.Enum;
			case "keyword":
				return completionKind.Keyword;
			case "snippet":
				return completionKind.Snippet;
			case "color":
				return completionKind.Color;
			case "file":
				return completionKind.File;
			case "reference":
				return completionKind.Reference;
			case "folder":
				return completionKind.Folder;
			case "enumMember":
				return completionKind.EnumMember;
			case "constant":
				return completionKind.Constant;
			case "struct":
				return completionKind.Struct;
			case "event":
				return completionKind.Event;
			case "operator":
				return completionKind.Operator;
			case "typeParameter":
				return completionKind.TypeParameter;
			default:
				return completionKind.Text;
		}
	}

	function sourceSymbolToDocumentSymbol(
		monaco: typeof Monaco,
		symbol: SourceSymbol
	): Monaco.languages.DocumentSymbol {
		const line = Math.max(1, symbol.line);
		const column = Math.max(1, symbol.column);
		const endColumn = column + Math.max(1, symbol.name.length);
		const range = new monaco.Range(line, column, line, endColumn);

		return {
			name: symbol.name,
			detail: symbol.detail,
			kind: sourceSymbolKind(monaco, symbol.kind),
			range,
			selectionRange: range,
		};
	}

	function sourceSymbolKind(monaco: typeof Monaco, kind: string): Monaco.languages.SymbolKind {
		const symbolKind = monaco.languages.SymbolKind;
		switch (kind) {
			case "namespace":
				return symbolKind.Namespace;
			case "class":
				return symbolKind.Class;
			case "interface":
				return symbolKind.Interface;
			case "enum":
				return symbolKind.Enum;
			case "function":
				return symbolKind.Function;
			case "method":
				return symbolKind.Method;
			case "constructor":
				return symbolKind.Constructor;
			case "property":
				return symbolKind.Property;
			case "constant":
				return symbolKind.Constant;
			case "record":
			case "struct":
				return symbolKind.Struct;
			case "type":
				return symbolKind.TypeParameter;
			case "variable":
			default:
				return symbolKind.Variable;
		}
	}

	function previewForModel(model: Monaco.editor.ITextModel): SourcePreview {
		const content = model.getValue();
		const path = model.uri.fsPath || model.uri.path;
		const fileName = path.split("/").filter(Boolean).at(-1) ?? "source";

		return {
			path,
			relativePath: fileName,
			fileName,
			language: model.getLanguageId(),
			byteCount: new TextEncoder().encode(content).length,
			content,
			lineCount: model.getLineCount(),
		};
	}

	function isSourceCodeLensSymbol(symbol: SourceSymbol) {
		if (!sourceCodeLensSymbolKinds.has(symbol.kind)) return false;
		if (symbol.kind !== "variable") return true;
		return /^(?:public|private|protected|internal|static|readonly|const|required|volatile|new)\b/.test(
			symbol.detail
		);
	}

	function sourceSymbolToLookupRequest(symbol: SourceSymbol): SourceEditorLookupRequest {
		const symbolNameOffset = symbol.detail.lastIndexOf(symbol.name);
		return {
			symbolName: symbol.name,
			line: symbol.line,
			column: symbolNameOffset >= 0 ? symbol.column + symbolNameOffset : symbol.column,
		};
	}

	function sourceCodeLensLookupRequest(
		codeLens: Monaco.languages.CodeLens
	): SourceEditorLookupRequest | null {
		const request = codeLens.command?.arguments?.[0] as SourceEditorLookupRequest | undefined;
		if (request?.symbolName && Number.isFinite(request.line) && Number.isFinite(request.column)) {
			return request;
		}

		return sourceCodeLensSpotFromId(codeLens.id);
	}

	async function sourceCodeLensReferenceCount(
		model: Monaco.editor.ITextModel,
		request: SourceEditorLookupRequest
	): Promise<SourceEditorReferenceCount | null> {
		const cacheKey = sourceCodeLensCountKey(model.uri.toString(), request);
		const remembered = codeLensReferenceCountCache.get(cacheKey);
		if (remembered && Date.now() - remembered.countedAt < sourceCodeLensCountMemoryMs) {
			return remembered.count === null
				? null
				: { count: remembered.count, atLeast: remembered.atLeast };
		}

		const lookup = await onReferenceCountLookup?.(request);
		// null/undefined ⇒ the count is unknown (no scan yet, or the scan ran out
		// of time). Cache and return null so the lens renders without a count
		// rather than "0".
		const next: SourceEditorReferenceCount | null =
			typeof lookup === "number"
				? { count: Math.max(0, lookup), atLeast: false }
				: lookup && typeof lookup.count === "number"
					? { count: Math.max(0, lookup.count), atLeast: lookup.atLeast === true }
					: null;
		codeLensReferenceCountCache.set(cacheKey, {
			count: next?.count ?? null,
			atLeast: next?.atLeast ?? false,
			countedAt: Date.now(),
		});
		return next;
	}

	// No "+" after fifty any more — a full pass gives the exact number. A pass
	// that could not read everything says "at least", because the tally covers
	// only the files it reached and the real number can only be higher.
	function formatReferenceCodeLensTitle(count: number, atLeast: boolean) {
		const word = count === 1 ? "reference" : "references";
		return atLeast ? `at least ${count} ${word}` : `${count} ${word}`;
	}

	function encodeSemanticTokens(tokens: SourceSemanticToken[]): Uint32Array {
		const data: number[] = [];
		let previousLine = 0;
		let previousStart = 0;

		for (const token of tokens) {
			const tokenTypeIndex = sourceSemanticTokenLegend.tokenTypes.indexOf(token.tokenType);
			if (tokenTypeIndex < 0 || token.length <= 0) continue;

			const line = Math.max(0, token.line - 1);
			const start = Math.max(0, token.startColumn - 1);
			data.push(line - previousLine, line === previousLine ? start - previousStart : start);
			data.push(token.length, tokenTypeIndex, 0);
			previousLine = line;
			previousStart = start;
		}

		return new Uint32Array(data);
	}

	function applyAppearance() {
		if (!monacoApi || !editor) return;

		// Start from the built-in appearance, then merge any user overrides that
		// are actually present. When `appearanceOverride` is undefined (or every
		// field is undefined) the resulting options are identical to the default
		// `sourcePreviewAppearance`, so an unconfigured editor is unchanged.
		const fontFamily = appearanceOverride?.fontFamily ?? sourcePreviewAppearance.fontFamily;
		const fontSize = appearanceOverride?.fontSize ?? sourcePreviewAppearance.fontSize;
		const lineHeight = appearanceOverride?.lineHeight ?? sourcePreviewAppearance.lineHeight;

		editor.updateOptions({
			fontFamily,
			fontLigatures: sourcePreviewAppearance.fontLigatures,
			fontSize,
			letterSpacing: sourcePreviewAppearance.letterSpacing,
			lineHeight,
			// The theme in force, NOT the shipped one: this runs again whenever the
			// font changes, and naming the shipped theme here would snap a switched
			// editor back to it mid-session.
			theme: activeTheme.monaco.id,
		});
		monacoApi.editor.setTheme(activeTheme.monaco.id);
	}

	function applyPreview() {
		if (!monacoApi || !editor || !preview) return;

		const language = monacoLanguageForSource(preview.language);
		const nextContent = content ?? preview.content;
		const uri = monacoApi.Uri.file(preview.path);
		let model = monacoApi.editor.getModel(uri);
		if (model?.isDisposed()) {
			ownedModels.delete(model);
			model = null;
		}

		if (!model) {
			model = monacoApi.editor.createModel(nextContent, language, uri);
			ownedModels.add(model);
		} else {
			if (model.getValue() !== nextContent) {
				setModelValue(model, nextContent);
			}
			if (model.getLanguageId() !== language) {
				monacoApi.editor.setModelLanguage(model, language);
			}
		}

		editor.updateOptions({
			domReadOnly: !editable,
			readOnly: !editable,
		});

		if (editor.getModel() !== model) {
			editor.setModel(model);
		}

		applyExternalDiagnostics(model);
		publishDiagnostics();
		publishSymbols();

		const pathChanged = currentPath !== preview.path;
		const targetLineChanged =
			currentTargetLine !== targetLine || currentTargetLineRequestId !== targetLineRequestId;
		if (targetLine && (pathChanged || targetLineChanged)) {
			const lineNumber = Math.min(Math.max(1, targetLine), model.getLineCount());
			currentPath = preview.path;
			currentTargetLine = targetLine;
			currentTargetLineRequestId = targetLineRequestId;
			revealTargetLine(model, lineNumber);
			return;
		}

		if (pathChanged) {
			currentPath = preview.path;
			currentTargetLine = null;
			currentTargetLineRequestId = targetLineRequestId;
			editor.setScrollTop(0);
			editor.setScrollLeft(0);
			editor.setPosition({ lineNumber: 1, column: 1 });
			return;
		}

		currentTargetLine = targetLine;
		currentTargetLineRequestId = targetLineRequestId;
	}

	function revealTargetLine(model: Monaco.editor.ITextModel, lineNumber: number) {
		if (!editor) return;

		const position = { lineNumber, column: 1 };
		editor.setPosition(position);
		editor.revealPositionInCenter(position);
		if (targetLineRevealFrame) window.cancelAnimationFrame(targetLineRevealFrame);
		targetLineRevealFrame = window.requestAnimationFrame(() => {
			targetLineRevealFrame = 0;
			if (editor?.getModel() !== model) return;
			editor.setPosition(position);
			editor.revealPositionInCenter(position);
		});
	}

	function setModelValue(model: Monaco.editor.ITextModel, nextContent: string) {
		applyingContent = true;
		try {
			model.setValue(nextContent);
		} finally {
			applyingContent = false;
		}
	}

	function handleEditorContentChange() {
		if (applyingContent || !editor) return;
		// The counted numbers are NOT dropped here. They count the name across the
		// project's saved files, which typing in this buffer does not change, and
		// dropping them on every keystroke re-asked for all of them at once.
		const nextContent = editor.getValue();
		onContentChange?.(nextContent);
		publishSymbols(nextContent);
	}

	function publishDiagnostics() {
		if (!monacoApi || !editor) return;

		const model = editor.getModel();
		if (!model) {
			onDiagnosticsChange?.([]);
			return;
		}

		const markers = monacoApi.editor.getModelMarkers({ resource: model.uri });
		onDiagnosticsChange?.(
			markers.map((marker) => ({
				severity: markerSeverityToSourceSeverity(marker.severity),
				message: marker.message,
				line: marker.startLineNumber,
				column: marker.startColumn,
				source: marker.source ?? undefined,
			}))
		);
	}

	function applyExternalDiagnostics(model = editor?.getModel()) {
		if (!monacoApi || !model) return;

		monacoApi.editor.setModelMarkers(
			model,
			"mcb-lsp",
			externalDiagnostics.map((diagnostic) => ({
				startLineNumber: Math.max(1, diagnostic.line),
				startColumn: Math.max(1, diagnostic.column),
				endLineNumber: Math.max(1, diagnostic.line),
				endColumn: Math.max(2, diagnostic.column + 1),
				message: diagnostic.message,
				severity: sourceDiagnosticToMarkerSeverity(diagnostic.severity),
				source: diagnostic.source ?? "lsp",
			}))
		);
	}

	function publishSymbols(nextContent = content ?? preview.content) {
		onSymbolsChange?.(extractSourceSymbols(preview, nextContent));
	}

	function markerSeverityToSourceSeverity(severity: Monaco.MarkerSeverity): SourceDiagnosticSeverity {
		if (!monacoApi) return "info";
		switch (severity) {
			case monacoApi.MarkerSeverity.Error:
				return "error";
			case monacoApi.MarkerSeverity.Warning:
				return "warning";
			case monacoApi.MarkerSeverity.Hint:
				return "hint";
			default:
				return "info";
		}
	}

	function sourceDiagnosticToMarkerSeverity(severity: SourceDiagnosticSeverity): Monaco.MarkerSeverity {
		if (!monacoApi) return 2 as Monaco.MarkerSeverity;
		switch (severity) {
			case "error":
				return monacoApi.MarkerSeverity.Error;
			case "warning":
				return monacoApi.MarkerSeverity.Warning;
			case "hint":
				return monacoApi.MarkerSeverity.Hint;
			default:
				return monacoApi.MarkerSeverity.Info;
		}
	}

	function runIntelligenceCommand() {
		if (!editor || !intelligenceCommand || intelligenceCommand.id === handledIntelligenceCommandId) {
			return;
		}

		handledIntelligenceCommandId = intelligenceCommand.id;
		if (intelligenceCommand.action === "definition") {
			requestDefinitionAtCursor();
			return;
		}
		if (intelligenceCommand.action === "references") {
			requestReferencesAtCursor();
			return;
		}
		if (intelligenceCommand.action === "implementation") {
			requestImplementationAtCursor();
			return;
		}
		if (intelligenceCommand.action === "type-definition") {
			requestTypeDefinitionAtCursor();
			return;
		}
		if (intelligenceCommand.action === "format") {
			requestFormatDocumentAtCursor();
			return;
		}
		if (intelligenceCommand.action === "rename") {
			requestRenameAtCursor();
			return;
		}
		if (intelligenceCommand.action === "quick-fix") {
			requestQuickFixAtCursor();
			return;
		}
		if (intelligenceCommand.action === "completion") {
			requestCompletionAtCursor();
			return;
		}
		if (intelligenceCommand.action === "signature-help") {
			requestSignatureHelpAtCursor();
			return;
		}

		requestHoverAtCursor();
	}

	function requestDefinitionAtCursor() {
		void editor?.getAction("editor.action.peekDefinition")?.run();
	}

	function requestDefinitionAtPosition(position: Monaco.IPosition) {
		editor?.setPosition(position);
		void editor?.getAction("editor.action.revealDefinition")?.run();
	}

	function requestReferencesAtCursor() {
		void editor?.getAction("editor.action.referenceSearch.trigger")?.run();
	}

	function requestReferencesAtPosition(position: Monaco.IPosition) {
		if (!editor) return;

		editor.setPosition(position);
		editor.revealPositionInCenterIfOutsideViewport(position);
		editor.focus();
		requestReferencesAtCursor();
	}

	function sourceEditorCommandService() {
		return (
			editor as unknown as { _commandService?: MonacoCommandService } | null
		)?._commandService ?? null;
	}

	function isMonacoCancellationError(value: unknown) {
		const error = value as { name?: unknown; message?: unknown } | null;
		return error?.name === "Canceled" && error.message === "Canceled";
	}

	function suppressMonacoCancellationErrorsBriefly() {
		if (monacoCancellationSuppressionTimer) {
			window.clearTimeout(monacoCancellationSuppressionTimer);
			monacoCancellationSuppressionTimer = 0;
		}
		monacoCancellationSuppressionDepth += 1;
		return () => {
			monacoCancellationSuppressionTimer = window.setTimeout(() => {
				monacoCancellationSuppressionDepth = Math.max(
					0,
					monacoCancellationSuppressionDepth - 1
				);
				monacoCancellationSuppressionTimer = 0;
			}, 120);
		};
	}

	function handleMonacoCancellationWindowError(event: ErrorEvent) {
		if (monacoCancellationSuppressionDepth === 0 || !isMonacoCancellationError(event.error)) {
			return;
		}

		event.preventDefault();
		event.stopImmediatePropagation();
	}

	function handleMonacoCancellationRejection(event: PromiseRejectionEvent) {
		if (monacoCancellationSuppressionDepth === 0 || !isMonacoCancellationError(event.reason)) {
			return;
		}

		event.preventDefault();
		event.stopImmediatePropagation();
	}

	function clearEditorNoticeTimers() {
		if (editorNoticeTimer) window.clearTimeout(editorNoticeTimer);
		if (editorNoticeDelayTimer) window.clearTimeout(editorNoticeDelayTimer);
		editorNoticeTimer = 0;
		editorNoticeDelayTimer = 0;
	}

	/**
	 * Show `text` in the corner of the editor. `clearAfterMs` of 0 leaves it up
	 * until the next call; pass a few seconds for a message the user only needs
	 * to read once. An empty `text` takes the notice away.
	 */
	function showEditorNotice(text: string, clearAfterMs = 0) {
		clearEditorNoticeTimers();
		editorNotice = text;
		if (!text || clearAfterMs <= 0) return;
		editorNoticeTimer = window.setTimeout(() => {
			editorNoticeTimer = 0;
			editorNotice = "";
		}, clearAfterMs);
	}

	/**
	 * Show `text` only if whatever is running is still running `delayMs` from
	 * now. Work that answers straight away should not make a message flash on
	 * screen; `showEditorNotice("")` cancels a message that never appeared.
	 */
	function showEditorNoticeAfter(delayMs: number, text: string) {
		clearEditorNoticeTimers();
		editorNoticeDelayTimer = window.setTimeout(() => {
			editorNoticeDelayTimer = 0;
			editorNotice = text;
		}, delayMs);
	}

	async function showCodeLensReferences(monaco: typeof Monaco, request: SourceEditorLookupRequest) {
		if (!editor) return;

		const position = {
			lineNumber: Math.max(1, request.line),
			column: Math.max(1, request.column),
		};
		const model = editor.getModel();
		editor.setPosition(position);
		editor.revealPositionInCenterIfOutsideViewport(position);
		editor.focus();

		// The lookup below can take seconds when the language server is busy, and
		// the only other thing that happens on click is the cursor moving — which
		// reads as "the link did nothing". Say what we are doing instead, once it
		// has taken long enough to be worth saying.
		showEditorNoticeAfter(250, "Finding references…");
		let targets: SourceReferenceTarget[] = [];
		try {
			targets = (await onReferenceLookup?.(request)) ?? [];
		} catch (error) {
			console.warn("Code lens reference lookup failed", error);
			showEditorNotice("Could not find references. Try again in a moment.", 5000);
			requestReferencesAtPosition(position);
			return;
		}
		showEditorNotice("");
		// A2: no bulk pre-read. peekLocations carries only {uri, range}; the peek tree
		// pulls each file group's preview model lazily on expand through our lazy
		// resolver shim (installLazyTargetModelResolver).
		const locations = targets.map((target) => sourceReferenceTargetToLocation(monaco, target));
		if (locations.length === 0 || !model) {
			requestReferencesAtPosition(position);
			return;
		}

		const commandService = sourceEditorCommandService();
		if (!commandService) {
			requestReferencesAtPosition(position);
			return;
		}

		try {
			await commandService.executeCommand(
				"editor.action.peekLocations",
				model.uri,
				position,
				locations,
				"peek"
			);
		} catch (error) {
			console.warn("Falling back to Monaco reference search after CodeLens peek failed", error);
			requestReferencesAtPosition(position);
		}
	}

	function runCodeLensReferenceCommand(monaco: typeof Monaco, request?: SourceEditorLookupRequest) {
		if (!request) return;
		void showCodeLensReferences(monaco, request);
	}

	function registerSourceCodeLensReferenceCommand(monaco: typeof Monaco) {
		codeLensReferenceCommandDisposable?.dispose();
		codeLensReferenceCommandDisposable = monaco.editor.registerCommand(
			codeLensReferenceCommandId,
			(_accessor, request?: SourceEditorLookupRequest) => {
				runCodeLensReferenceCommand(monaco, request);
			}
		);
	}

	function installExternalEditorOpener(monaco: typeof Monaco) {
		editorOpenerDisposable?.dispose();
		editorOpenerDisposable = monaco.editor.registerEditorOpener({
			openCodeEditor: async (source, resource, selectionOrPosition) => {
				if (!editor || source !== editor || !onExternalNavigation) return false;

				const path = sourcePathFromMonacoUri(resource);
				if (!path || path === currentPath) return false;

				// A2 / design §4.3: a single go-to-definition jump needs exactly ONE
				// model — the navigated target. The provider returned locations only (no
				// fan-out), so materialize just this one on demand now (cached/deduped via
				// ensureSourceTargetModel). If the read fails we still return true and let
				// onExternalNavigation drive the load — the jump must never silently fail.
				let targetModel = monaco.editor.getModel(resource);
				if (!targetModel) {
					targetModel = await ensureSourceTargetModel(monaco, { path } as SourceRecord);
				}

				const location = locationFromMonacoSelection(selectionOrPosition);
				const position = {
					lineNumber: location.line,
					column: location.column
				};
				if (targetModel) {
					editor.setModel(targetModel);
					editor.setPosition(position);
					editor.revealPositionInCenterIfOutsideViewport(position);
					editor.focus();
				}

				const releaseCancellationSuppression = suppressMonacoCancellationErrorsBriefly();
				queueMicrotask(() => {
					void Promise.resolve(onExternalNavigation({ path, ...location })).finally(
						releaseCancellationSuppression
					);
				});
				return true;
			}
		});
	}

	function requestImplementationAtCursor() {
		const implementationAction = editor?.getAction("editor.action.peekImplementation");
		if (implementationAction) {
			void implementationAction.run();
			return;
		}

		void editor?.getAction("editor.action.goToImplementation")?.run();
	}

	function requestTypeDefinitionAtCursor() {
		const typeDefinitionAction = editor?.getAction("editor.action.peekTypeDefinition");
		if (typeDefinitionAction) {
			void typeDefinitionAction.run();
			return;
		}

		void editor?.getAction("editor.action.goToTypeDefinition")?.run();
	}

	function requestFormatDocumentAtCursor() {
		void editor?.getAction("editor.action.formatDocument")?.run();
	}

	function requestRenameAtCursor() {
		void editor?.getAction("editor.action.rename")?.run();
	}

	function requestQuickFixAtCursor() {
		void editor?.getAction("editor.action.quickFix")?.run();
	}

	function requestCompletionAtCursor() {
		void editor?.getAction("editor.action.triggerSuggest")?.run();
	}

	function requestSignatureHelpAtCursor() {
		void editor?.getAction("editor.action.triggerParameterHints")?.run();
	}

	function requestHoverAtCursor() {
		void editor?.getAction("editor.action.showHover")?.run();
	}

	function requestProblemNavigationAtCursor(direction: 1 | -1) {
		const position = editor?.getPosition();
		if (!position) return;

		const request = {
			line: position.lineNumber,
			column: position.column,
		};

		if (direction === 1) {
			onNextProblemRequest?.(request);
			return;
		}

		onPreviousProblemRequest?.(request);
	}

	function lookupRequestAtCursor(): SourceEditorLookupRequest | null {
		const position = editor?.getPosition();
		if (!position) return null;

		return lookupRequestAtPosition(position);
	}

	function lookupRequestAtPosition(position: Monaco.IPosition): SourceEditorLookupRequest | null {
		const model = editor?.getModel();
		if (!model) return null;

		return lookupRequestForModelPosition(model, position);
	}

	function lookupRequestForModelPosition(
		model: Monaco.editor.ITextModel,
		position: Monaco.IPosition
	): SourceEditorLookupRequest | null {
		if (!model) return null;

		const word = model.getWordAtPosition(position);
		if (!word) return null;

		return {
			symbolName: word.word,
			line: position.lineNumber,
			column: word.startColumn,
		};
	}

	function completionRequestForModelPosition(
		model: Monaco.editor.ITextModel,
		position: Monaco.IPosition
	): SourceEditorLookupRequest {
		const word = model.getWordUntilPosition(position);
		return {
			symbolName: word.word,
			line: position.lineNumber,
			column: position.column,
		};
	}

	function positionRequestForModelPosition(
		model: Monaco.editor.ITextModel,
		position: Monaco.IPosition
	): SourceEditorLookupRequest {
		const word = model.getWordUntilPosition(position);
		return {
			symbolName: word.word,
			line: position.lineNumber,
			column: position.column,
		};
	}

	async function waitForConnectedMountHost(mountHost: HTMLDivElement) {
		if (mountHost.isConnected) return true;

		await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
		return !componentDestroyed && host === mountHost && mountHost.isConnected;
	}

	onMount(async () => {
		const mountHost = host;
		if (!mountHost) return;

		window.addEventListener("error", handleMonacoCancellationWindowError);
		window.addEventListener("unhandledrejection", handleMonacoCancellationRejection);
		installWorker();

		const modules = await Promise.all([
			import("monaco-editor/esm/vs/editor/editor.api"),
			import("monaco-editor/esm/vs/basic-languages/cpp/cpp.contribution"),
			import("monaco-editor/esm/vs/basic-languages/csharp/csharp.contribution"),
			import("monaco-editor/esm/vs/basic-languages/css/css.contribution"),
			import("monaco-editor/esm/vs/basic-languages/dart/dart.contribution"),
			import("monaco-editor/esm/vs/basic-languages/dockerfile/dockerfile.contribution"),
			import("monaco-editor/esm/vs/basic-languages/fsharp/fsharp.contribution"),
			import("monaco-editor/esm/vs/basic-languages/go/go.contribution"),
			import("monaco-editor/esm/vs/basic-languages/graphql/graphql.contribution"),
			import("monaco-editor/esm/vs/basic-languages/hcl/hcl.contribution"),
			import("monaco-editor/esm/vs/basic-languages/html/html.contribution"),
			import("monaco-editor/esm/vs/basic-languages/ini/ini.contribution"),
			import("monaco-editor/esm/vs/basic-languages/java/java.contribution"),
			import("monaco-editor/esm/vs/basic-languages/javascript/javascript.contribution"),
			import("monaco-editor/esm/vs/basic-languages/kotlin/kotlin.contribution"),
			import("monaco-editor/esm/vs/basic-languages/less/less.contribution"),
			import("monaco-editor/esm/vs/basic-languages/lua/lua.contribution"),
			import("monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution"),
			import("monaco-editor/esm/vs/basic-languages/mdx/mdx.contribution"),
			import("monaco-editor/esm/vs/basic-languages/php/php.contribution"),
			import("monaco-editor/esm/vs/basic-languages/powershell/powershell.contribution"),
			import("monaco-editor/esm/vs/basic-languages/protobuf/protobuf.contribution"),
			import("monaco-editor/esm/vs/basic-languages/python/python.contribution"),
			import("monaco-editor/esm/vs/basic-languages/razor/razor.contribution"),
			import("monaco-editor/esm/vs/basic-languages/ruby/ruby.contribution"),
			import("monaco-editor/esm/vs/basic-languages/rust/rust.contribution"),
			import("monaco-editor/esm/vs/basic-languages/scss/scss.contribution"),
			import("monaco-editor/esm/vs/basic-languages/shell/shell.contribution"),
			import("monaco-editor/esm/vs/basic-languages/sql/sql.contribution"),
			import("monaco-editor/esm/vs/basic-languages/swift/swift.contribution"),
			import("monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution"),
			import("monaco-editor/esm/vs/basic-languages/xml/xml.contribution"),
			import("monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution"),
			import("monaco-editor/esm/vs/language/json/monaco.contribution"),
			import("monaco-editor/esm/vs/language/typescript/monaco.contribution"),
		]);
		if (
			componentDestroyed ||
			host !== mountHost ||
			!(await waitForConnectedMountHost(mountHost))
		) {
			return;
		}
		const monaco = modules[0] as typeof Monaco;
		const typeScriptLanguage = modules[modules.length - 1] as TypeScriptContribution;

		monacoApi = monaco;
		configureMonaco(monaco);
		configureTypeScriptLanguageService(typeScriptLanguage);
		registerSourceSemanticTokens(monaco);
		registerSourceHoverProvider(monaco);
		registerSourceDefinitionProvider(monaco);
		registerSourceDocumentHighlightProvider(monaco);
		registerSourceImplementationProvider(monaco);
		registerSourceTypeDefinitionProvider(monaco);
		registerSourceFormattingProvider(monaco);
		registerSourceRenameProvider(monaco);
		registerSourceCodeActionProvider(monaco);
		registerSourceSignatureHelpProvider(monaco);
		registerSourceInlayHintsProvider(monaco);
		registerSourceReferenceProvider(monaco);
		registerSourceCompletionProvider(monaco);
		registerSourceDocumentSymbolProvider(monaco);

		editor = monaco.editor.create(mountHost, {
			automaticLayout: false,
			bracketPairColorization: { enabled: true },
			contextmenu: true,
			cursorBlinking: "solid",
			cursorStyle: "line-thin",
			cursorWidth: 1,
			domReadOnly: !editable,
			folding: true,
			fontFamily: sourcePreviewAppearance.fontFamily,
			fontLigatures: sourcePreviewAppearance.fontLigatures,
			fontSize: sourcePreviewAppearance.fontSize,
			glyphMargin: false,
			hideCursorInOverviewRuler: true,
			language: monacoLanguageForSource(preview.language),
			letterSpacing: sourcePreviewAppearance.letterSpacing,
			lineDecorationsWidth: 14,
			lineHeight: sourcePreviewAppearance.lineHeight,
			lineNumbers: "on",
			lineNumbersMinChars: 3,
			minimap: { enabled: false },
			model: null,
			occurrencesHighlight: "singleFile",
			overviewRulerBorder: false,
			overviewRulerLanes: 0,
			padding: { top: 16, bottom: 20 },
			readOnly: !editable,
			renderLineHighlight: "gutter",
			renderWhitespace: "selection",
			scrollBeyondLastLine: false,
			scrollbar: {
				alwaysConsumeMouseWheel: false,
				horizontalScrollbarSize: 12,
				verticalScrollbarSize: 12,
			},
			"semanticHighlighting.enabled": true,
			inlayHints: {
				enabled: "on",
			},
			codeLens: true,
			smoothScrolling: true,
			stickyScroll: { enabled: true },
			tabSize: 4,
			theme: activeTheme.monaco.id,
			wordWrap: "off",
		});

		externalWorkspaceEditCommandId =
			editor.addCommand(0, (_accessor, action?: SourceCodeAction) => {
				if (action) void onWorkspaceEditAction?.(action);
			}) ?? "";
		registerSourceCodeLensReferenceCommand(monaco);
		registerSourceCodeLensProvider(monaco);
		installExternalEditorOpener(monaco);
		installLazyTargetModelResolver(monaco);

		editorActionDisposables = [
			editor.addAction({
				id: codeLensReferenceCommandId,
				label: "Find CodeLens References",
				run: (_editor, request?: SourceEditorLookupRequest) =>
					runCodeLensReferenceCommand(monaco, request),
			}),
			editor.addAction({
				id: "mcb.source.goToDefinition",
				label: "Go to Definition",
				keybindings: [monaco.KeyCode.F12],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 1,
				run: () => requestDefinitionAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.findReferences",
				label: "Find References",
				keybindings: [monaco.KeyMod.Shift | monaco.KeyCode.F12],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 2,
				run: () => requestReferencesAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.findImplementations",
				label: "Find Implementations",
				contextMenuGroupId: "navigation",
				contextMenuOrder: 2.5,
				run: () => requestImplementationAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.goToTypeDefinition",
				label: "Go to Type Definition",
				contextMenuGroupId: "navigation",
				contextMenuOrder: 2.6,
				run: () => requestTypeDefinitionAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.showHover",
				label: "Show Hover",
				contextMenuGroupId: "navigation",
				contextMenuOrder: 3,
				run: () => requestHoverAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.formatDocument",
				label: "Format Document",
				contextMenuGroupId: "1_modification",
				contextMenuOrder: 0.5,
				run: () => requestFormatDocumentAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.renameSymbol",
				label: "Rename Symbol",
				keybindings: [monaco.KeyCode.F2],
				contextMenuGroupId: "1_modification",
				contextMenuOrder: 0.6,
				run: () => requestRenameAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.quickFix",
				label: "Quick Fix",
				keybindings: [monaco.KeyMod.Alt | monaco.KeyCode.Enter],
				contextMenuGroupId: "1_modification",
				contextMenuOrder: 0.7,
				run: () => requestQuickFixAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.triggerCompletions",
				label: "Trigger Completions",
				contextMenuGroupId: "navigation",
				contextMenuOrder: 3.1,
				run: () => requestCompletionAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.showSignatureHelp",
				label: "Show Signature Help",
				contextMenuGroupId: "navigation",
				contextMenuOrder: 3.2,
				run: () => requestSignatureHelpAtCursor(),
			}),
			editor.addAction({
				id: "mcb.source.save",
				label: "Save",
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
				contextMenuGroupId: "1_modification",
				contextMenuOrder: 1,
				run: () => onSaveRequest?.(),
			}),
			editor.addAction({
				id: "mcb.source.quickOpen",
				label: "Open File",
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyP],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0,
				run: () => onQuickOpenRequest?.(),
			}),
			editor.addAction({
				id: "mcb.source.commandPalette",
				label: "Command Palette",
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0.1,
				run: () => onCommandPaletteRequest?.(),
			}),
			editor.addAction({
				id: "mcb.source.goToLine",
				label: "Go to Line",
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyG],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0.2,
				run: () => onGoToLineRequest?.(),
			}),
			editor.addAction({
				id: "mcb.source.navigateBack",
				label: "Go Back",
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.BracketLeft],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0.21,
				run: () => onNavigateBackRequest?.(),
			}),
			editor.addAction({
				id: "mcb.source.navigateForward",
				label: "Go Forward",
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.BracketRight],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0.22,
				run: () => onNavigateForwardRequest?.(),
			}),
			editor.addAction({
				id: "mcb.source.nextProblem",
				label: "Next Problem",
				keybindings: [monaco.KeyCode.F8],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0.41,
				run: () => requestProblemNavigationAtCursor(1),
			}),
			editor.addAction({
				id: "mcb.source.previousProblem",
				label: "Previous Problem",
				keybindings: [monaco.KeyMod.Shift | monaco.KeyCode.F8],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0.42,
				run: () => requestProblemNavigationAtCursor(-1),
			}),
		];

		layoutObserver = new ResizeObserver(() => {
			if (layoutFrame) window.cancelAnimationFrame(layoutFrame);
			layoutFrame = window.requestAnimationFrame(() => {
				layoutFrame = 0;
				editor?.layout();
			});
		});
		layoutObserver.observe(host);

		contentChangeDisposable = editor.onDidChangeModelContent(handleEditorContentChange);
		modelChangeDisposable = editor.onDidChangeModel(() => {
			codeLensReferenceCountCache.clear();
			publishDiagnostics();
		});
		markerChangeDisposable = monaco.editor.onDidChangeMarkers((uris) => {
			const modelUri = editor?.getModel()?.uri.toString();
			if (modelUri && uris.some((uri) => uri.toString() === modelUri)) {
				publishDiagnostics();
			}
		});
		mouseDefinitionDisposable = editor.onMouseDown((event) => {
			if (
				event.target.type !== monaco.editor.MouseTargetType.CONTENT_TEXT ||
				!event.target.position ||
				(!event.event.browserEvent.metaKey && !event.event.browserEvent.ctrlKey)
			) {
				return;
			}

			event.event.preventDefault();
			event.event.stopPropagation();
			requestDefinitionAtPosition(event.target.position);
		});
		isReady = true;
		applyAppearance();
		applyPreview();
		runIntelligenceCommand();
	});

	$effect(() => {
		if (isReady) {
			applyPreview();
			runIntelligenceCommand();
		}
	});

	// Re-apply appearance whenever the user-provided overrides change. Reading
	// the fields here registers them as dependencies. No-op until the editor is
	// ready; when no override is set this re-runs applyAppearance() with the
	// default values, leaving the editor visually unchanged.
	$effect(() => {
		void appearanceOverride?.fontSize;
		void appearanceOverride?.fontFamily;
		void appearanceOverride?.lineHeight;
		if (isReady) {
			applyAppearance();
		}
	});

	onDestroy(() => {
		componentDestroyed = true;
		// A torn-down editor must stop being repainted, or the theme service keeps
		// a dead Monaco in its set for the life of the page.
		disposeThemeApplier();
		if (layoutFrame) {
			window.cancelAnimationFrame(layoutFrame);
			layoutFrame = 0;
		}
		if (targetLineRevealFrame) {
			window.cancelAnimationFrame(targetLineRevealFrame);
			targetLineRevealFrame = 0;
		}
		window.removeEventListener("error", handleMonacoCancellationWindowError);
		window.removeEventListener("unhandledrejection", handleMonacoCancellationRejection);
		if (monacoCancellationSuppressionTimer) {
			window.clearTimeout(monacoCancellationSuppressionTimer);
			monacoCancellationSuppressionTimer = 0;
		}
		monacoCancellationSuppressionDepth = 0;
		clearEditorNoticeTimers();
		editorNotice = "";
		layoutObserver?.disconnect();
		layoutObserver = null;
		contentChangeDisposable?.dispose();
		modelChangeDisposable?.dispose();
		markerChangeDisposable?.dispose();
		mouseDefinitionDisposable?.dispose();
		semanticTokensDisposable?.dispose();
		hoverProviderDisposable?.dispose();
		definitionProviderDisposable?.dispose();
		documentHighlightProviderDisposable?.dispose();
		implementationProviderDisposable?.dispose();
		typeDefinitionProviderDisposable?.dispose();
		formattingProviderDisposable?.dispose();
		renameProviderDisposable?.dispose();
		codeActionProviderDisposable?.dispose();
		signatureHelpProviderDisposable?.dispose();
		inlayHintsProviderDisposable?.dispose();
		referenceProviderDisposable?.dispose();
		codeLensProviderDisposable?.dispose();
		completionProviderDisposable?.dispose();
		documentSymbolProviderDisposable?.dispose();
		codeLensReferenceCommandDisposable?.dispose();
		editorOpenerDisposable?.dispose();
		uninstallLazyTargetModelResolver();
		codeLensReferenceCommandDisposable = null;
		editorOpenerDisposable = null;
		for (const disposable of editorActionDisposables) {
			disposable.dispose();
		}
		editorActionDisposables = [];
		onDiagnosticsChange?.([]);
		onSymbolsChange?.([]);
		if (monacoApi) {
			const model = editor?.getModel();
			if (model) monacoApi.editor.setModelMarkers(model, "mcb-lsp", []);
		}
		editor?.dispose();
		externalWorkspaceEditCommandId = "";
		codeLensReferenceCountCache.clear();
		for (const model of ownedModels) {
			model.dispose();
		}
		ownedModels.clear();
		externalTargetModelLru.clear();
		inFlightTargetModels.clear();
	});
</script>

<div
	class="source-editor"
	data-font-family={sourcePreviewAppearance.fontFamily}
	data-testid="monaco-source-editor"
	data-theme-id={activeTheme.monaco.id}
	style={`--source-editor-background: ${editorBackground}`}
>
	<div bind:this={host} class="monaco-host"></div>

	{#if loading || !isReady}
		<div class="skeleton-code" aria-label="Loading source preview">
			{#each Array.from({ length: 13 }) as _, index}
				<span style={`--line-width: ${index % 4 === 0 ? 48 : index % 3 === 0 ? 66 : 86}%`}></span>
			{/each}
		</div>
	{/if}

	{#if editorNotice}
		<div class="editor-notice" aria-live="polite">{editorNotice}</div>
	{/if}
</div>

<style>
	.source-editor {
		position: relative;
		height: 100%;
		min-height: 0;
		overflow: hidden;
		background: var(--source-editor-background, #17191e);
	}

	.monaco-host {
		width: 100%;
		height: 100%;
	}

	.monaco-host :global(.monaco-editor),
	.monaco-host :global(.overflow-guard) {
		border-bottom-left-radius: 13px;
		border-bottom-right-radius: 13px;
	}

	.monaco-host :global(.monaco-editor .margin) {
		background: var(--source-editor-background, #17191e);
	}

	.editor-notice {
		position: absolute;
		right: 18px;
		bottom: 14px;
		z-index: 6;
		max-width: 60%;
		padding: 6px 12px;
		border: 1px solid #343841;
		border-radius: 999px;
		background: #23262d;
		color: #cfd3dc;
		font-size: 12px;
		line-height: 1.4;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		box-shadow: 0 6px 18px rgba(0, 0, 0, 0.35);
		pointer-events: none;
	}

	.skeleton-code {
		position: absolute;
		inset: 0;
		display: grid;
		align-content: start;
		gap: 12px;
		padding: 22px;
		background: var(--source-editor-background, #17191e);
	}

	.skeleton-code span {
		display: block;
		width: var(--line-width);
		height: 13px;
		border-radius: 999px;
		background: linear-gradient(90deg, #23262d, #343841, #23262d);
		background-size: 180% 100%;
		animation: shimmer 1.2s ease-in-out infinite;
	}

	@keyframes shimmer {
		from {
			background-position: 100% 0;
		}
		to {
			background-position: -80% 0;
		}
	}
</style>
