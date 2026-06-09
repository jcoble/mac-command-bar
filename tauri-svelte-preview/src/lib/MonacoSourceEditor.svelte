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

	type SourceEditorIntelligenceAction = "definition" | "hover";

	type SourceEditorIntelligenceCommand = {
		id: number;
		action: SourceEditorIntelligenceAction;
	};

	type TypeScriptContribution = typeof import("monaco-editor/esm/vs/language/typescript/monaco.contribution");

	type Props = {
		preview: SourcePreview;
		content?: string;
		editable?: boolean;
		loading?: boolean;
		targetLine?: number | null;
		targetLineRequestId?: number;
		intelligenceCommand?: SourceEditorIntelligenceCommand | null;
		onContentChange?: (content: string) => void;
		onDiagnosticsChange?: (diagnostics: SourceDiagnostic[]) => void;
		onSymbolsChange?: (symbols: SourceSymbol[]) => void;
	};

	let {
		preview,
		content,
		editable = false,
		loading = false,
		targetLine = null,
		targetLineRequestId = 0,
		intelligenceCommand = null,
		onContentChange,
		onDiagnosticsChange,
		onSymbolsChange,
	}: Props = $props();

	let host = $state<HTMLDivElement | null>(null);
	let editor = $state<Monaco.editor.IStandaloneCodeEditor | null>(null);
	let monacoApi: typeof Monaco | null = null;
	let contentChangeDisposable: Monaco.IDisposable | null = null;
	let markerChangeDisposable: Monaco.IDisposable | null = null;
	let semanticTokensDisposable: Monaco.IDisposable | null = null;
	let currentPath = "";
	let currentTargetLine: number | null = null;
	let currentTargetLineRequestId = -1;
	let handledIntelligenceCommandId = -1;
	let applyingContent = false;
	let isReady = $state(false);
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

	function runIntelligenceCommand() {
		if (!editor || !intelligenceCommand || intelligenceCommand.id === handledIntelligenceCommandId) {
			return;
		}

		handledIntelligenceCommandId = intelligenceCommand.id;
		const actionId =
			intelligenceCommand.action === "hover"
				? "editor.action.showHover"
				: "editor.action.revealDefinition";
		void editor.getAction(actionId)?.run();
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

		editor = monaco.editor.create(host, {
			automaticLayout: true,
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

		contentChangeDisposable = editor.onDidChangeModelContent(handleEditorContentChange);
		markerChangeDisposable = monaco.editor.onDidChangeMarkers((uris) => {
			const modelUri = editor?.getModel()?.uri.toString();
			if (modelUri && uris.some((uri) => uri.toString() === modelUri)) {
				publishDiagnostics();
			}
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
		contentChangeDisposable?.dispose();
		markerChangeDisposable?.dispose();
		semanticTokensDisposable?.dispose();
		onDiagnosticsChange?.([]);
		onSymbolsChange?.([]);
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
