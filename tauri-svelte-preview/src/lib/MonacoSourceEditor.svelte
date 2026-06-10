<script lang="ts">
	import type * as Monaco from "monaco-editor/esm/vs/editor/editor.api";
	import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
	import JsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
	import TypeScriptWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
	import "monaco-editor/min/vs/editor/editor.main.css";
	import { onDestroy, onMount } from "svelte";
	import { sourcePreviewAppearance } from "./sourcePreviewAppearance";
	import {
		extractSourceSemanticTokens,
		extractSourceSymbols,
		monacoLanguageForSource,
		sourceSemanticTokenLegend,
		type SourceDiagnostic,
		type SourceDiagnosticSeverity,
		type SourcePreview,
		type SourceSemanticToken,
		type SourceSymbol,
	} from "./sourceData";

	type SourceEditorIntelligenceAction = "definition" | "hover" | "references";

	type SourceEditorIntelligenceCommand = {
		id: number;
		action: SourceEditorIntelligenceAction;
	};

	type SourceEditorLookupRequest = {
		symbolName: string;
		line: number;
		column: number;
	};

	type SourceEditorHoverResult = {
		contents: string[];
	};

	type TypeScriptContribution = typeof import("monaco-editor/esm/vs/language/typescript/monaco.contribution");

	type Props = {
		preview: SourcePreview;
		content?: string;
		editable?: boolean;
		externalDiagnostics?: SourceDiagnostic[];
		loading?: boolean;
		targetLine?: number | null;
		targetLineRequestId?: number;
		intelligenceCommand?: SourceEditorIntelligenceCommand | null;
		onContentChange?: (content: string) => void;
		onCommandPaletteRequest?: () => void;
		onDiagnosticsChange?: (diagnostics: SourceDiagnostic[]) => void;
		onDefinitionLookup?: (request: SourceEditorLookupRequest) => void;
		onGoToLineRequest?: () => void;
		onHoverLookup?: (request: SourceEditorLookupRequest) => SourceEditorHoverResult | Promise<SourceEditorHoverResult | null> | null;
		onProblemsRequest?: () => void;
		onQuickOpenRequest?: () => void;
		onReferenceLookup?: (request: SourceEditorLookupRequest) => void;
		onSaveRequest?: () => void;
		onSymbolsRequest?: () => void;
		onSymbolsChange?: (symbols: SourceSymbol[]) => void;
	};

	let {
		preview,
		content,
		editable = false,
		externalDiagnostics = [],
		loading = false,
		targetLine = null,
		targetLineRequestId = 0,
		intelligenceCommand = null,
		onContentChange,
		onCommandPaletteRequest,
		onDiagnosticsChange,
		onDefinitionLookup,
		onGoToLineRequest,
		onHoverLookup,
		onProblemsRequest,
		onQuickOpenRequest,
		onReferenceLookup,
		onSaveRequest,
		onSymbolsRequest,
		onSymbolsChange,
	}: Props = $props();

	let host = $state<HTMLDivElement | null>(null);
	let editor = $state<Monaco.editor.IStandaloneCodeEditor | null>(null);
	let monacoApi: typeof Monaco | null = null;
	let contentChangeDisposable: Monaco.IDisposable | null = null;
	let markerChangeDisposable: Monaco.IDisposable | null = null;
	let mouseDefinitionDisposable: Monaco.IDisposable | null = null;
	let semanticTokensDisposable: Monaco.IDisposable | null = null;
	let hoverProviderDisposable: Monaco.IDisposable | null = null;
	let editorActionDisposables: Monaco.IDisposable[] = [];
	let currentPath = "";
	let currentTargetLine: number | null = null;
	let currentTargetLineRequestId = -1;
	let handledIntelligenceCommandId = -1;
	let applyingContent = false;
	let isReady = $state(false);
	let layoutObserver: ResizeObserver | null = null;
	let layoutFrame = 0;
	const ownedModels = new Set<Monaco.editor.ITextModel>();
	const editorBackground = sourcePreviewAppearance.theme.colors["editor.background"] ?? "#17191e";

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

	function configureMonaco(monaco: typeof Monaco) {
		const { id, ...theme } = sourcePreviewAppearance.theme;
		monaco.editor.defineTheme(id, {
			...theme,
			rules: theme.rules ?? [],
			colors: theme.colors ?? {},
		});
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
		const diagnosticsOptions = {
			noSemanticValidation: false,
			noSyntaxValidation: false,
			noSuggestionDiagnostics: false,
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
			["typescript", "javascript", "csharp"],
			{
				getLegend: () => ({
					tokenTypes: [...sourceSemanticTokenLegend.tokenTypes],
					tokenModifiers: [...sourceSemanticTokenLegend.tokenModifiers],
				}),
				provideDocumentSemanticTokens: (model) => ({
					data: encodeSemanticTokens(
						extractSourceSemanticTokens(previewForModel(model), model.getValue())
					),
				}),
				releaseDocumentSemanticTokens: () => {},
			}
		);
	}

	function registerSourceHoverProvider(monaco: typeof Monaco) {
		hoverProviderDisposable?.dispose();
		hoverProviderDisposable = monaco.languages.registerHoverProvider(
			["typescript", "javascript", "csharp"],
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

		editor.updateOptions({
			fontFamily: sourcePreviewAppearance.fontFamily,
			fontLigatures: sourcePreviewAppearance.fontLigatures,
			fontSize: sourcePreviewAppearance.fontSize,
			letterSpacing: sourcePreviewAppearance.letterSpacing,
			lineHeight: sourcePreviewAppearance.lineHeight,
			theme: sourcePreviewAppearance.theme.id,
		});
		monacoApi.editor.setTheme(sourcePreviewAppearance.theme.id);
	}

	function applyPreview() {
		if (!monacoApi || !editor || !preview) return;

		const language = monacoLanguageForSource(preview.language);
		const nextContent = content ?? preview.content;
		const uri = monacoApi.Uri.file(preview.path);
		let model = monacoApi.editor.getModel(uri);

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
			editor.setPosition({ lineNumber, column: 1 });
			editor.revealLineInCenterIfOutsideViewport(lineNumber);
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

		requestHoverAtCursor();
	}

	function requestDefinitionAtCursor() {
		const request = lookupRequestAtCursor();
		if (!request) return;

		onDefinitionLookup?.(request);
		void editor?.getAction("editor.action.revealDefinition")?.run();
	}

	function requestDefinitionAtPosition(position: Monaco.IPosition) {
		const request = lookupRequestAtPosition(position);
		if (!request) return;

		editor?.setPosition(position);
		onDefinitionLookup?.(request);
		void editor?.getAction("editor.action.revealDefinition")?.run();
	}

	function requestReferencesAtCursor() {
		const request = lookupRequestAtCursor();
		if (!request) return;

		onReferenceLookup?.(request);
		void editor?.getAction("editor.action.referenceSearch.trigger")?.run();
	}

	function requestHoverAtCursor() {
		void editor?.getAction("editor.action.showHover")?.run();
	}

	function lookupRequestAtCursor(): SourceEditorLookupRequest | null {
		const position = editor?.getPosition();
		if (!position) return null;

		return lookupRequestAtPosition(position);
	}

	function lookupRequestAtPosition(position: Monaco.IPosition): SourceEditorLookupRequest | null {
		const model = editor?.getModel();
		if (!model) return null;

		const word = model.getWordAtPosition(position);
		if (!word) return null;

		return {
			symbolName: word.word,
			line: position.lineNumber,
			column: word.startColumn,
		};
	}

	onMount(async () => {
		if (!host) return;

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
		const monaco = modules[0] as typeof Monaco;
		const typeScriptLanguage = modules[modules.length - 1] as TypeScriptContribution;

		monacoApi = monaco;
		configureMonaco(monaco);
		configureTypeScriptLanguageService(typeScriptLanguage);
		registerSourceSemanticTokens(monaco);
		registerSourceHoverProvider(monaco);

		editor = monaco.editor.create(host, {
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
			occurrencesHighlight: "off",
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
			smoothScrolling: true,
			stickyScroll: { enabled: false },
			tabSize: 4,
			theme: sourcePreviewAppearance.theme.id,
			wordWrap: "off",
		});

		editorActionDisposables = [
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
				id: "mcb.source.showHover",
				label: "Show Hover",
				contextMenuGroupId: "navigation",
				contextMenuOrder: 3,
				run: () => requestHoverAtCursor(),
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
				id: "mcb.source.showSymbols",
				label: "Show Symbols",
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyO],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0.3,
				run: () => onSymbolsRequest?.(),
			}),
			editor.addAction({
				id: "mcb.source.showProblems",
				label: "Show Problems",
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyM],
				contextMenuGroupId: "navigation",
				contextMenuOrder: 0.4,
				run: () => onProblemsRequest?.(),
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

	onDestroy(() => {
		if (layoutFrame) {
			window.cancelAnimationFrame(layoutFrame);
			layoutFrame = 0;
		}
		layoutObserver?.disconnect();
		layoutObserver = null;
		contentChangeDisposable?.dispose();
		markerChangeDisposable?.dispose();
		mouseDefinitionDisposable?.dispose();
		semanticTokensDisposable?.dispose();
		hoverProviderDisposable?.dispose();
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
		for (const model of ownedModels) {
			model.dispose();
		}
		ownedModels.clear();
	});
</script>

<div
	class="source-editor"
	data-font-family={sourcePreviewAppearance.fontFamily}
	data-testid="monaco-source-editor"
	data-theme-id={sourcePreviewAppearance.theme.id}
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
