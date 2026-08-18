<script lang="ts">
	import type * as Monaco from "monaco-editor/esm/vs/editor/editor.api";
	import { PRODUCT_NAME } from "$lib/productIdentity";
	import "@codingame/monaco-vscode-api/vscode/vs/editor/contrib/gotoSymbol/browser/goToCommands";
	// Deep imports for the lazy target-model resolver (Task A2, design-monaco.md §4.2,
	// Route 1). The standalone `ITextModelService` is a global eager singleton
	// (standaloneServices.js: registerSingleton(ITextModelService, …, Eager)) and is
	// the exact instance the reference-peek tree resolves preview models through
	// (referencesWidget.js __param(4, ITextModelService) → DataSource →
	// FileReferences.resolve → createModelReference). Overriding its on-miss behaviour
	// makes peek read each file group lazily on expand instead of eagerly up front.
	import { ITextModelService, StandaloneServices } from "@codingame/monaco-vscode-api/services";
	import { onDestroy, onMount } from "svelte";
	import {
		formatSourceCodeLensTitle,
		settledSourceCodeLensCount,
		sourceCodeLensCountKey,
		sourceCodeLensId,
	} from "./sourceCodeLensKeys";
	import {
		dotnetWorkspaceCommandIds,
		dotnetWorkspaceLensTitles,
		type DotnetWorkspaceAction,
	} from "./workspaceCodeLens";
	import { sourcePreviewAppearance } from "./sourcePreviewAppearance";
	import { animateWhenVisible } from "$lib/shell/elementVisibility";
	import { listThemes } from "$lib/shell/themes/themeRegistry";
	import { currentTheme, registerMonacoApplier } from "$lib/shell/themes/themeService";
	import {
		configureMonacoWorkers,
		monacoVscodeApiIsInitialized,
	} from "$lib/shell/editor/monacoWorkers";
	import {
		editorStartFailureMessage,
		waitForConnectedHost,
		EDITOR_START_LIMIT_MS,
	} from "$lib/shell/components/editor/editorStartup";
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
		/**
		 * Which file the spot is in. Only the margin's row spots carry it, so
		 * that whoever counts references can refuse to answer when the number
		 * would be about a file other than the one on screen.
		 */
		filePath?: string;
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
	//
	// Three endings matter, and they are told apart by WHEN and WITH WHAT the
	// promise settles. Still pending means the counting is going on and the
	// margin says so. A number means show it. `null` means whatever was asked
	// cannot tell us, and the margin shows nothing at all rather than a number
	// nobody counted.
	type SourceEditorReferenceCount = { count: number; atLeast: boolean };
	type SourceEditorReferenceCountLookup = (
		request: SourceEditorLookupRequest
	) =>
		| number
		| SourceEditorReferenceCount
		| Promise<number | SourceEditorReferenceCount | null>
		| null
		| undefined;

	// Where the margin rows sit. The editor can work these out itself by reading
	// the text, but it only knows how to read TypeScript, JavaScript and C#;
	// whoever supplies this callback may know the shape of other languages.
	// Answering null means "I cannot", and the editor reads the text as before.
	type SourceEditorCodeLensAnchorLookup = (
		preview: SourcePreview
	) => SourceSymbol[] | Promise<SourceSymbol[] | null> | null | undefined;

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

	type TypeScriptContribution = typeof import("@codingame/monaco-vscode-standalone-typescript-language-features");

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
		nativeCsharpLanguageClient?: boolean;
		loading?: boolean;
		targetLine?: number | null;
		targetLineRequestId?: number;
		intelligenceCommand?: SourceEditorIntelligenceCommand | null;
		onContentChange?: (content: string) => void;
		onCodeActionLookup?: SourceEditorCodeActionLookup;
		onCodeLensAnchorLookup?: SourceEditorCodeLensAnchorLookup;
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
		/**
		 * This file was edited, so whoever counts references should count its
		 * symbols again. Sent once the reader has stopped typing for a moment,
		 * with the path of the file that changed and nothing else — every other
		 * file's numbers still stand.
		 */
		onReferenceCountsOutOfDate?: (filePath: string) => void;
		onDotnetBuildRequest?: () => void | Promise<void>;
		onDotnetTestRequest?: () => void | Promise<void>;
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
		nativeCsharpLanguageClient = false,
		loading = false,
		targetLine = null,
		targetLineRequestId = 0,
		intelligenceCommand = null,
		onContentChange,
		onCodeActionLookup,
		onCodeLensAnchorLookup,
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
		onReferenceCountsOutOfDate,
		onDotnetBuildRequest,
		onDotnetTestRequest,
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
	/**
	 * Why the editor never appeared, in a sentence — shown INSTEAD of the
	 * placeholder bars. Nothing but `isReady` can end those bars, so every way
	 * of failing to become ready has to land here or the reader is left looking
	 * at a shimmer for the rest of the session with no idea what went wrong.
	 */
	let startError = $state<string | null>(null);
	/** True once starting has been given up on, so waits stop waiting. */
	let gaveUpStarting = false;
	let startLimitTimer = 0;
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
	let dotnetCodeLensCommandDisposables: Monaco.IDisposable[] = [];
	let monacoCancellationSuppressionDepth = 0;
	let monacoCancellationSuppressionTimer = 0;
	let reconciledNativeCsharpMode: boolean | null = null;
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
	type ModelReference<T> = {
		object: T;
		dispose(): void;
	};

	type StandaloneTextModelResolver = {
		createModelReference: (
			resource: Monaco.Uri
		) => Promise<ModelReference<{ object: Monaco.editor.ITextModel }>>;
	};
	let textModelResolverService: StandaloneTextModelResolver | null = null;
	let originalCreateModelReference:
		| StandaloneTextModelResolver["createModelReference"]
		| null = null;
	// ── The "N references" row above each symbol ─────────────────────────────
	//
	// The row is drawn the instant the symbols in the file are known, saying it
	// is counting, and each real number replaces that line as it arrives —
	// working down the page rather than making the reader wait for the last one.
	// Nothing here ever puts a number on screen that was not handed to us.
	//
	// This map is only a paint cache, keyed by model URI. It survives model
	// switches within this editor instance so a warm file can draw its number
	// synchronously; `onReferenceCountLookup` owns durable freshness.
	//
	// `undefined` for a spot means nobody has answered yet — the row says it is
	// counting. `null` means we asked and were told the number cannot be worked
	// out, and the row shows nothing at all.
	const codeLensCounts = new Map<string, SourceEditorReferenceCount | null>();
	/** Spots already asked about, so repainting the rows does not ask again. */
	const codeLensAsked = new Set<string>();
	/** Latest asynchronous count request allowed to repaint each row. */
	const codeLensCountRequestIds = new Map<string, number>();
	let codeLensCountRequestSequence = 0;
	/**
	 * Authoritative symbol anchors, kept with the real file model they describe.
	 *
	 * Without this URI-keyed cache Monaco first drew rows from its quick text
	 * parser, then replaced their identities when Roslyn answered. A real count
	 * could therefore appear, disappear behind a fresh placeholder, and appear
	 * again. Native files now wait for that one authoritative anchor answer;
	 * switching tabs reuses it.
	 */
	const codeLensNamedSymbolsByModel = new Map<string, SourceSymbol[] | null>();
	const codeLensNamedSymbolsPending = new Map<string, symbol>();
	/** Last read of the symbols in the file, so scrolling does not re-read it. */
	let codeLensSpotsCache: {
		modelUri: string;
		versionId: number;
		spots: SourceEditorLookupRequest[];
	} | null = null;
	/** Tells Monaco a row's line has changed, so it draws the rows again. */
	let codeLensChangeEmitter: Monaco.Emitter<Monaco.languages.CodeLensProvider> | null = null;
	let sourceCodeLensProvider: Monaco.languages.CodeLensProvider | null = null;
	let codeLensRepaintTimer = 0;
	let codeLensRecountTimer = 0;
	/** When the reader last typed, so counting waits until they pause. */
	let codeLensLastEditAt = 0;
	let editorScrollDisposable: Monaco.IDisposable | null = null;
	/**
	 * How long the rows wait after a number lands before they are drawn again.
	 * Numbers arrive a few at a time, and redrawing the whole margin for each
	 * one separately is wasted work nobody can see.
	 */
	const CODE_LENS_REPAINT_SETTLE_MS = 80;
	/**
	 * How long after the reader stops typing before the file's symbols are
	 * counted again. Editing a file can genuinely change how many places use
	 * the things in it, but asking on every keystroke is exactly the storm this
	 * whole design exists to avoid.
	 */
	const CODE_LENS_RECOUNT_AFTER_EDIT_MS = 2000;
	/** Most rows one file gets, however many symbols it has. */
	const MAX_CODE_LENS_ROWS = 120;
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
		if (!monacoVscodeApiIsInitialized()) {
			monacoApi?.editor.setTheme(theme.monaco.id);
		}
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
	function customProviderLanguageIDs(): string[] {
		return nativeCsharpLanguageClient
			? sourceLspMonacoLanguageIDs.filter((language) => language !== "csharp")
			: sourceLspMonacoLanguageIDs;
	}
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
		configureMonacoWorkers();
	}

	/**
	 * Define every theme the shell ships, and hand Monaco's repainter to the theme
	 * service so a switch reaches the editor. Runs once per Monaco load, which is
	 * the right scope: `setTheme` is global to Monaco, so one registration covers
	 * however many editors exist.
	 */
	function configureMonaco(monaco: typeof Monaco) {
		if (monacoVscodeApiIsInitialized()) return;
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
		codeLensChangeEmitter?.dispose();
		codeLensChangeEmitter = new monaco.Emitter<Monaco.languages.CodeLensProvider>();

		sourceCodeLensProvider = {
			onDidChange: codeLensChangeEmitter.event,
			provideCodeLenses: (model: Monaco.editor.ITextModel) => {
				// Only the file the reader is looking at gets rows. A model in a
				// peek window is some other file, and every number we can get is
				// worked out against the file on screen — so a number put above a
				// peeked file's symbol would be about the wrong file entirely.
				if (!editor || editor.getModel() !== model) {
					return { lenses: [], dispose() {} };
				}

				const workspaceLenses = dotnetWorkspaceCodeLenses(monaco, model);
				// Reference counts are intentionally disconnected unless a caller
				// explicitly supplies the custom counter. This keeps the retained
				// implementation dormant while the VS Code-compatible CodeLens path
				// is evaluated, and prevents placeholder "0 references" rows.
				if (!onReferenceCountLookup) {
					return { lenses: workspaceLenses, dispose() {} };
				}

				requestCodeLensNamedSymbols(model);
				const modelUri = model.uri.toString();
				const waitingForAuthoritativeAnchors =
					Boolean(onCodeLensAnchorLookup) &&
					codeLensNamedSymbolsPending.has(modelUri) &&
					!codeLensNamedSymbolsByModel.has(modelUri);
				const spots = waitingForAuthoritativeAnchors ? [] : codeLensSpotsForModel(model);
				askForCountsWorthAskingFor(model, spots);

				return {
					lenses: [
						...workspaceLenses,
						...spots.map((spot) => ({
							id: sourceCodeLensId(spot),
							range: new monaco.Range(spot.line, 1, spot.line, 1),
							command: codeLensCommandForSpot(model, spot),
						})),
					],
					dispose() {},
				};
			},
			// Every row above already carries its line, so Monaco only ever asks
			// this about a row we deliberately left blank — one whose number we
			// were told cannot be worked out. It stays blank.
			resolveCodeLens: (
				_model: Monaco.editor.ITextModel,
				codeLens: Monaco.languages.CodeLens
			) => codeLens,
		};

		codeLensProviderDisposable = monaco.languages.registerCodeLensProvider(
			customProviderLanguageIDs(),
			sourceCodeLensProvider
		);
	}

	function dotnetWorkspaceCodeLenses(
		monaco: typeof Monaco,
		model: Monaco.editor.ITextModel
	): Monaco.languages.CodeLens[] {
		if (model.getLanguageId() !== "csharp") return [];

		const actions: DotnetWorkspaceAction[] = [];
		if (onDotnetBuildRequest) actions.push("build");
		if (onDotnetTestRequest) actions.push("test");
		return actions.map((action) => ({
			id: `${dotnetWorkspaceCommandIds[action]}:${model.uri.toString()}`,
			range: new monaco.Range(1, 1, 1, 1),
			command: {
				id: dotnetWorkspaceCommandIds[action],
				title: dotnetWorkspaceLensTitles[action],
			},
		}));
	}

	/**
	 * The symbols this file's rows sit above. The ones named by whoever supplied
	 * `onCodeLensAnchorLookup`, when they could name any; otherwise the editor's
	 * own reading of the text, which is all it has for the browser preview and
	 * for a desktop build too old to be asked.
	 */
	function codeLensSpotsForModel(
		model: Monaco.editor.ITextModel
	): SourceEditorLookupRequest[] {
		const modelUri = model.uri.toString();
		const versionId = model.getVersionId();
		const lastRead = codeLensSpotsCache;
		if (lastRead && lastRead.modelUri === modelUri && lastRead.versionId === versionId) {
			return lastRead.spots;
		}

		const named = codeLensNamedSymbolsByModel.get(modelUri) ?? null;
		const symbols = named ?? extractSourceSymbols(previewForModel(model), model.getValue());
		const filePath = model.uri.fsPath || model.uri.path;
		const spots = symbols
			.filter(isSourceCodeLensSymbol)
			.slice(0, MAX_CODE_LENS_ROWS)
			// The file travels with the spot so that whoever counts it can refuse
			// to answer when the number would be about a different file.
			.map((symbol) => ({ ...sourceSymbolToLookupRequest(symbol), filePath }));

		codeLensSpotsCache = { modelUri, versionId, spots };
		return spots;
	}

	/** What one row says right now. */
	function codeLensCommandForSpot(
		model: Monaco.editor.ITextModel,
		spot: SourceEditorLookupRequest
	): Monaco.languages.Command | undefined {
		const counted = codeLensCounts.get(sourceCodeLensCountKey(model.uri.toString(), spot));
		// Match VS Code: an unresolved Roslyn request has no CodeLens row. A real
		// settled zero is still displayed below as "0 references".
		if (counted === undefined) return undefined;
		// We asked and were told the number cannot be worked out. Nothing on the
		// row beats a wrong number on it.
		if (counted === null) return undefined;

		return {
			id: codeLensReferenceCommandId,
			title: formatSourceCodeLensTitle(counted.count, counted.atLeast),
			arguments: [spot],
		};
	}

	/**
	 * Every symbol worth counting in this file.
	 *
	 * The source-intelligence service batches these names into one project
	 * scan. Limiting the requests to the viewport made untouched rows claim
	 * they were "counting" even though no question had been asked, and forced a
	 * new request after every scroll.
	 */
	function codeLensSpotsWorthCountingNow(
		spots: SourceEditorLookupRequest[]
	): SourceEditorLookupRequest[] {
		return spots;
	}

	function askForCountsWorthAskingFor(
		model: Monaco.editor.ITextModel,
		spots: SourceEditorLookupRequest[]
	) {
		if (!onReferenceCountLookup || spots.length === 0) return;
		// Mid-edit. The lines are moving under the reader's hands, so asking now
		// would ask about spots that will not exist a keystroke later. The pause
		// after they stop typing is what sets this going again.
		if (codeLensLastEditAt !== 0 && Date.now() - codeLensLastEditAt < CODE_LENS_RECOUNT_AFTER_EDIT_MS) {
			return;
		}

		const modelUri = model.uri.toString();
		for (const spot of codeLensSpotsWorthCountingNow(spots)) {
			const key = sourceCodeLensCountKey(modelUri, spot);
			if (codeLensAsked.has(key)) continue;
			codeLensAsked.add(key);
			void askForOneCount(modelUri, key, spot);
		}
	}

	function askForOneCount(
		modelUri: string,
		key: string,
		spot: SourceEditorLookupRequest
	) {
		const requestId = ++codeLensCountRequestSequence;
		codeLensCountRequestIds.set(key, requestId);
		let answer:
			| number
			| SourceEditorReferenceCount
			| Promise<number | SourceEditorReferenceCount | null>
			| null
			| undefined;
		try {
			answer = onReferenceCountLookup?.(spot);
		} catch {
			acceptCountAnswer(modelUri, key, requestId, null);
			return;
		}

		if (isPromiseLike<number | SourceEditorReferenceCount | null>(answer)) {
			void Promise.resolve(answer)
				.then((settled) => acceptCountAnswer(modelUri, key, requestId, settled))
				.catch(() => acceptCountAnswer(modelUri, key, requestId, null));
			return;
		}
		acceptCountAnswer(modelUri, key, requestId, answer);
	}

	function isPromiseLike<T>(value: unknown): value is PromiseLike<T> {
		return Boolean(
			value && typeof (value as { then?: unknown }).then === "function"
		);
	}

	function acceptCountAnswer(
		modelUri: string,
		key: string,
		requestId: number,
		answer: number | SourceEditorReferenceCount | null | undefined
	) {
		// The reader opened another file while we were waiting. This number is
		// about a file that is no longer on screen.
		if (componentDestroyed || editor?.getModel()?.uri.toString() !== modelUri) return;
		const isLatestRequest = codeLensCountRequestIds.get(key) === requestId;
		if (!isLatestRequest) return;
		codeLensCountRequestIds.delete(key);

		const settled = settledSourceCodeLensCount(
			codeLensCounts.get(key),
			referenceCountFromAnswer(answer),
			isLatestRequest
		);
		if (settled === undefined) codeLensCounts.delete(key);
		else codeLensCounts.set(key, settled);
		drawTheRowsAgainSoon();
	}

	/** A count may arrive as a bare number from an older caller. */
	function referenceCountFromAnswer(
		answer: number | SourceEditorReferenceCount | null | undefined
	): SourceEditorReferenceCount | null {
		if (typeof answer === "number") return { count: Math.max(0, answer), atLeast: false };
		if (answer && typeof answer.count === "number") {
			return { count: Math.max(0, answer.count), atLeast: answer.atLeast === true };
		}
		return null;
	}

	function drawTheRowsAgainSoon() {
		if (codeLensRepaintTimer || componentDestroyed) return;
		codeLensRepaintTimer = window.setTimeout(() => {
			codeLensRepaintTimer = 0;
			if (componentDestroyed || !sourceCodeLensProvider) return;
			codeLensChangeEmitter?.fire(sourceCodeLensProvider);
		}, CODE_LENS_REPAINT_SETTLE_MS);
	}

	/** Ask, once per file, for a better list of symbols than the text gives us. */
	function requestCodeLensNamedSymbols(model: Monaco.editor.ITextModel) {
		if (!onCodeLensAnchorLookup) return;
		const modelUri = model.uri.toString();
		if (
			codeLensNamedSymbolsByModel.has(modelUri) ||
			codeLensNamedSymbolsPending.has(modelUri)
		) {
			return;
		}

		const requestToken = Symbol(modelUri);
		codeLensNamedSymbolsPending.set(modelUri, requestToken);
		void (async () => {
			let named: SourceSymbol[] | null | undefined = null;
			try {
				named = await onCodeLensAnchorLookup?.(previewForModel(model));
			} catch {
				named = null;
			}

			if (componentDestroyed) return;
			// A reload may have invalidated this request and started a newer one.
			// Its answer is the only one allowed to name the rows.
			if (codeLensNamedSymbolsPending.get(modelUri) !== requestToken) return;
			codeLensNamedSymbolsPending.delete(modelUri);

			// Remembered either way: an answer of "I cannot" is an answer, and
			// asking again on every repaint would be a call per scroll.
			codeLensNamedSymbolsByModel.set(modelUri, named?.length ? named : null);
			if (editor?.getModel()?.uri.toString() !== modelUri) return;
			codeLensSpotsCache = null;
			drawTheRowsAgainSoon();
		})();
	}

	/**
	 * Count this file's symbols again, once the reader has stopped typing for a
	 * moment. The numbers already on screen stay where they are until better
	 * ones arrive, so the margin does not blink back to zero every time the
	 * reader pauses.
	 */
	function countAgainOnceTheReaderStops() {
		codeLensLastEditAt = Date.now();
		if (codeLensRecountTimer) window.clearTimeout(codeLensRecountTimer);
		codeLensRecountTimer = window.setTimeout(() => {
			codeLensRecountTimer = 0;
			codeLensLastEditAt = 0;
			if (componentDestroyed) return;

			const model = editor?.getModel();
			if (!model) return;

			onReferenceCountsOutOfDate?.(model.uri.fsPath || model.uri.path);
			codeLensAsked.clear();
			forgetNumbersForSymbolsThatAreGone(model);
			// The list of symbols is deliberately NOT asked for again here.
			// Whoever names them for us reads the file as it is saved on disk, so
			// asking again mid-edit would put the rows back on the lines they had
			// before the reader started typing. The editor's own reading of the
			// text follows the buffer, and a re-read from disk asks again.
			drawTheRowsAgainSoon();
		}, CODE_LENS_RECOUNT_AFTER_EDIT_MS);
	}

	/**
	 * Drop the numbers for symbols that are no longer where they were.
	 *
	 * Every row is named by the line it sits on, so adding a line above one
	 * gives everything below it a new name — and the old names would otherwise
	 * pile up for as long as the file stayed open. This runs after the reader
	 * pauses, when the lines have settled.
	 */
	function forgetNumbersForSymbolsThatAreGone(model: Monaco.editor.ITextModel) {
		const modelUri = model.uri.toString();
		const stillHere = new Set(
			codeLensSpotsForModel(model).map((spot) => sourceCodeLensCountKey(modelUri, spot))
		);
		for (const key of [...codeLensCounts.keys()]) {
			if (!stillHere.has(key)) codeLensCounts.delete(key);
		}
	}

	/** Start over: another file is on screen, or this one was reloaded. */
	function forgetCodeLensRows(clearRememberedCounts = false) {
		if (clearRememberedCounts) {
			codeLensCounts.clear();
			codeLensNamedSymbolsByModel.clear();
			codeLensNamedSymbolsPending.clear();
		}
		codeLensAsked.clear();
		codeLensCountRequestIds.clear();
		codeLensSpotsCache = null;
		codeLensLastEditAt = 0;
		if (codeLensRepaintTimer) {
			window.clearTimeout(codeLensRepaintTimer);
			codeLensRepaintTimer = 0;
		}
		if (codeLensRecountTimer) {
			window.clearTimeout(codeLensRecountTimer);
			codeLensRecountTimer = 0;
		}
	}

	function registerSourceDocumentHighlightProvider(monaco: typeof Monaco) {
		documentHighlightProviderDisposable?.dispose();
		documentHighlightProviderDisposable = monaco.languages.registerDocumentHighlightProvider(
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
			{
				displayName: `${PRODUCT_NAME} LSP`,
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
			customProviderLanguageIDs(),
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
			customProviderLanguageIDs(),
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
			tags: [],
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
			...(monacoVscodeApiIsInitialized()
				? {}
				: { theme: activeTheme.monaco.id }),
		});
		if (!monacoVscodeApiIsInitialized()) {
			monacoApi.editor.setTheme(activeTheme.monaco.id);
		}
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
				// The file itself changed under us — re-read from disk, or written.
				// Both the symbols in it and how many places use them can have
				// moved, so both are worth asking about again. The numbers already
				// on screen stay until better ones arrive.
				const modelUri = model.uri.toString();
				codeLensNamedSymbolsByModel.delete(modelUri);
				codeLensNamedSymbolsPending.delete(modelUri);
				codeLensAsked.clear();
				codeLensSpotsCache = null;
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
		// The numbers already on screen are NOT dropped here — they stay until
		// better ones arrive. What this starts is the pause: nothing is counted
		// again until the reader has stopped typing for a moment.
		const nextContent = editor.getValue();
		onContentChange?.(nextContent);
		publishSymbols(nextContent);
		countAgainOnceTheReaderStops();
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
		// The reference lookup and the CodeLens count are one semantic answer on
		// the service side. Read the now-cached count back immediately so a click
		// can never open populated Peek rows while the margin still says zero.
		if (model) {
			const modelUri = model.uri.toString();
			const key = sourceCodeLensCountKey(modelUri, request);
			codeLensAsked.add(key);
			askForOneCount(modelUri, key, request);
		}
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

	async function runDotnetWorkspaceCommand(action: DotnetWorkspaceAction) {
		const request = action === "build" ? onDotnetBuildRequest : onDotnetTestRequest;
		if (!request) return;

		showEditorNoticeAfter(150, `Starting .NET ${action}…`);
		try {
			await request();
			showEditorNotice("");
		} catch (error) {
			console.warn(`Could not start .NET ${action}`, error);
			showEditorNotice(`Could not start .NET ${action}.`, 5000);
		}
	}

	function registerDotnetWorkspaceCodeLensCommands(monaco: typeof Monaco) {
		for (const disposable of dotnetCodeLensCommandDisposables) disposable.dispose();
		dotnetCodeLensCommandDisposables = (["build", "test"] as DotnetWorkspaceAction[]).map(
			(action) =>
				monaco.editor.registerCommand(
					dotnetWorkspaceCommandIds[action],
					() => void runDotnetWorkspaceCommand(action)
				)
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

	/**
	 * Wait until the element the editor is built into is back in the document.
	 *
	 * The shell moves the center surfaces into their dock hosts AFTER the panels
	 * inside them have mounted, so at launch — with a file already open — this
	 * element is off the document for a stretch of frames. Waiting one frame and
	 * giving up was what left the placeholder bars on screen with nothing behind
	 * them. The time limit on starting is what ends this wait if it never can.
	 */
	function waitForConnectedMountHost(mountHost: HTMLDivElement) {
		return waitForConnectedHost({
			isConnected: () => mountHost.isConnected,
			isWanted: () => !componentDestroyed && !gaveUpStarting && host === mountHost,
			nextFrame: () =>
				new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()))
		});
	}

	/**
	 * Transfer C# feature ownership without replacing the editor or its active model.
	 *
	 * The Monaco language registries are global, while the native Roslyn client becomes
	 * ready after the editor has already painted. Re-registering the existing custom
	 * providers changes only their language selector: native mode excludes C#, and
	 * fallback mode includes it again. The custom CodeLens commands and lazy resolver
	 * follow the same ownership boundary. A repeated value is a strict no-op.
	 */
	function reconcileLanguageProviderOwnership(monaco: typeof Monaco, nativeMode: boolean) {
		if (reconciledNativeCsharpMode === nativeMode) return;

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

		if (nativeMode) {
			codeLensReferenceCommandDisposable?.dispose();
			codeLensReferenceCommandDisposable = null;
			for (const disposable of dotnetCodeLensCommandDisposables) disposable.dispose();
			dotnetCodeLensCommandDisposables = [];
			codeLensProviderDisposable?.dispose();
			codeLensProviderDisposable = null;
			codeLensChangeEmitter?.dispose();
			codeLensChangeEmitter = null;
			sourceCodeLensProvider = null;
			uninstallLazyTargetModelResolver();
		} else {
			if (onReferenceCountLookup) registerSourceCodeLensReferenceCommand(monaco);
			registerDotnetWorkspaceCodeLensCommands(monaco);
			registerSourceCodeLensProvider(monaco);
			installLazyTargetModelResolver(monaco);
		}

		reconciledNativeCsharpMode = nativeMode;
	}

	/**
	 * Build the editor, or say why it could not be built.
	 *
	 * Everything in here is wrapped, because the placeholder bars end in exactly
	 * one place — `isReady` — and a throw on the way there (a chunk that will not
	 * download, services that came up half-initialized) used to become an
	 * unhandled rejection nobody sees. The time limit covers the other half: work
	 * that neither finishes nor fails. Either way the reader gets a sentence and
	 * a way to try again rather than a shimmer that never stops.
	 */
	async function startEditor(mountHost: HTMLDivElement): Promise<void> {
		startError = null;
		gaveUpStarting = false;
		if (startLimitTimer) window.clearTimeout(startLimitTimer);
		startLimitTimer = window.setTimeout(() => {
			startLimitTimer = 0;
			// "It never finished" is the weakest thing that can be said, so it is
			// only ever said when there is nothing better: an editor that is up, or
			// a failure that already named itself, both leave this alone.
			if (componentDestroyed || isReady || startError) return;
			gaveUpStarting = true;
			startError = editorStartFailureMessage(preview.relativePath);
		}, EDITOR_START_LIMIT_MS);

		try {
			await buildEditor(mountHost);
		} catch (error) {
			if (componentDestroyed) return;
			gaveUpStarting = true;
			startError = editorStartFailureMessage(preview.relativePath, error);
			if (startLimitTimer) {
				window.clearTimeout(startLimitTimer);
				startLimitTimer = 0;
			}
		} finally {
			// Only a finished editor cancels the limit. Returning without one and
			// without a message is the case the limit exists for.
			if (isReady && startLimitTimer) {
				window.clearTimeout(startLimitTimer);
				startLimitTimer = 0;
			}
		}
	}

	/** Throw away a half-built editor and try the whole thing again. */
	function retryStartingEditor(): void {
		const mountHost = host;
		if (!mountHost || componentDestroyed) return;
		for (const disposable of editorActionDisposables) disposable.dispose();
		editorActionDisposables = [];
		editor?.dispose();
		editor = null;
		// Provider ownership is registered during the build, so it has to be
		// decided again rather than remembered from the attempt that failed.
		reconciledNativeCsharpMode = null;
		void startEditor(mountHost);
	}

	async function buildEditor(mountHost: HTMLDivElement) {
		window.addEventListener("error", handleMonacoCancellationWindowError);
		window.addEventListener("unhandledrejection", handleMonacoCancellationRejection);
		installWorker();

		const vscodeServicesReady = monacoVscodeApiIsInitialized();
		const [monaco, _standaloneLanguages, _jsonLanguage, typeScriptLanguage] = await Promise.all([
			import("monaco-editor/esm/vs/editor/editor.api"),
			vscodeServicesReady
				? Promise.resolve(null)
				: import("@codingame/monaco-vscode-standalone-languages"),
			import("@codingame/monaco-vscode-standalone-json-language-features"),
			import("@codingame/monaco-vscode-standalone-typescript-language-features"),
		]);
		if (
			componentDestroyed ||
			host !== mountHost ||
			!(await waitForConnectedMountHost(mountHost))
		) {
			return;
		}
		monacoApi = monaco;
		configureMonaco(monaco);
		if (typeScriptLanguage) {
			configureTypeScriptLanguageService(typeScriptLanguage);
		}
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
			...(monacoVscodeApiIsInitialized()
				? {}
				: { theme: activeTheme.monaco.id }),
			wordWrap: "off",
		});

		externalWorkspaceEditCommandId =
			editor.addCommand(0, (_accessor, action?: SourceCodeAction) => {
				if (action) void onWorkspaceEditAction?.(action);
			}) ?? "";
		reconcileLanguageProviderOwnership(monaco, nativeCsharpLanguageClient);
		installExternalEditorOpener(monaco);

		editorActionDisposables = [
			...(onReferenceCountLookup
				? [
						editor.addAction({
							id: codeLensReferenceCommandId,
							label: "Find CodeLens References",
							run: (_editor, request?: SourceEditorLookupRequest) =>
								runCodeLensReferenceCommand(monaco, request),
						}),
					]
				: []),
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
			// Keys include the model URI, so numbers for the file being left can
			// stay in memory and repaint immediately when its tab comes back.
			forgetCodeLensRows();
			publishDiagnostics();
		});
		// Scrolling brings symbols into view that were too far away to count
		// before. Drawing the rows again is what asks about them.
		editorScrollDisposable = editor.onDidScrollChange(() => drawTheRowsAgainSoon());
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
		if (startLimitTimer) {
			window.clearTimeout(startLimitTimer);
			startLimitTimer = 0;
		}
		applyAppearance();
		applyPreview();
		runIntelligenceCommand();
	}

	onMount(() => {
		const mountHost = host;
		if (mountHost) void startEditor(mountHost);
	});

	$effect(() => {
		if (isReady) {
			applyPreview();
			runIntelligenceCommand();
		}
	});

	$effect(() => {
		const nativeMode = nativeCsharpLanguageClient;
		if (isReady && monacoApi) {
			reconcileLanguageProviderOwnership(monacoApi, nativeMode);
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
		if (startLimitTimer) {
			window.clearTimeout(startLimitTimer);
			startLimitTimer = 0;
		}
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
		editorScrollDisposable?.dispose();
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
		codeLensChangeEmitter?.dispose();
		codeLensChangeEmitter = null;
		sourceCodeLensProvider = null;
		completionProviderDisposable?.dispose();
		documentSymbolProviderDisposable?.dispose();
		codeLensReferenceCommandDisposable?.dispose();
		for (const disposable of dotnetCodeLensCommandDisposables) disposable.dispose();
		dotnetCodeLensCommandDisposables = [];
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
		reconciledNativeCsharpMode = null;
		externalWorkspaceEditCommandId = "";
		forgetCodeLensRows(true);
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

	<!-- The bars mean "any moment now". They are shown only while that is still
	     true: once starting has been given up on, the reason takes their place. -->
	{#if startError}
		<div class="start-error" role="alert">
			<p>{startError}</p>
			<button type="button" class="start-retry" onclick={retryStartingEditor}>Try again</button>
		</div>
	{:else if loading || !isReady}
		<div class="skeleton-code" use:animateWhenVisible aria-label="Loading source preview">
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

	.start-error {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 0 24px;
		background: var(--source-editor-background, #17191e);
		color: #f2999b;
		font-size: 13px;
		line-height: 1.5;
		text-align: center;
	}

	.start-error p {
		margin: 0;
		max-width: 52ch;
	}

	.start-retry {
		border: 1px solid #343841;
		border-radius: 5px;
		background: #23262d;
		color: #cfd3dc;
		cursor: pointer;
		font-family: var(--font-mono);
		font-size: 12px;
		padding: 3px 10px;
	}

	.start-retry:hover {
		background: #2b2f37;
		color: #ffffff;
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
		position: relative;
		display: block;
		overflow: hidden;
		width: var(--line-width);
		height: 13px;
		border-radius: 999px;
		background: #23262d;
	}

	/* The sweep is a highlight that slides across the bar. Moving a gradient's
	   background-position instead, which is what this used to do, repaints the
	   whole bar every frame — thirteen bars at sixty frames a second, for as
	   long as the editor takes to start, and forever if it never does. A
	   transform is handed to the compositor and costs the main thread nothing.

	   It also stops entirely once the bars scroll or tab out of sight: an
	   animation off screen is still animated, and still charged for. */
	.skeleton-code span::after {
		content: '';
		position: absolute;
		inset: 0;
		background: linear-gradient(90deg, transparent, #343841, transparent);
		animation: shimmer 1.2s ease-in-out infinite;
		animation-play-state: var(--motion-state, running);
	}

	@keyframes shimmer {
		from {
			transform: translateX(-100%);
		}
		to {
			transform: translateX(100%);
		}
	}
</style>
