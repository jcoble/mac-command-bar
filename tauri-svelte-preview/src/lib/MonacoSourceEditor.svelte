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
		type SourceCodeAction,
		type SourceCodeActionLookupRequest,
		type SourceCompletionItem,
		type SourceDiagnostic,
		type SourceDiagnosticSeverity,
		type SourceDefinitionTarget,
		type SourceDocumentHighlight,
		type SourceInlayHint,
		type SourcePreview,
		type SourceReferenceTarget,
		type SourceRenameResult,
		type SourceSemanticToken,
		type SourceSignatureHelp,
		type SourceSymbol,
		type SourceTextEdit,
	} from "./sourceData";

	type SourceEditorIntelligenceAction =
		| "definition"
		| "hover"
		| "implementation"
		| "format"
		| "quick-fix"
		| "references"
		| "rename"
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

	type SourceEditorHoverResult = {
		contents: string[];
	};

	type SourceEditorDefinitionLookup = (
		request: SourceEditorLookupRequest
	) => SourceDefinitionTarget[] | Promise<SourceDefinitionTarget[]> | null | undefined;

	type SourceEditorReferenceLookup = (
		request: SourceEditorLookupRequest
	) => SourceReferenceTarget[] | Promise<SourceReferenceTarget[]> | null | undefined;

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
		onCodeActionLookup?: SourceEditorCodeActionLookup;
		onCommandPaletteRequest?: () => void;
		onCompletionLookup?: SourceEditorCompletionLookup;
		onDiagnosticsChange?: (diagnostics: SourceDiagnostic[]) => void;
		onDefinitionLookup?: SourceEditorDefinitionLookup;
		onDocumentHighlightLookup?: SourceEditorDocumentHighlightLookup;
		onFormatDocument?: SourceEditorFormatDocument;
		onGoToLineRequest?: () => void;
		onHoverLookup?: (request: SourceEditorLookupRequest) => SourceEditorHoverResult | Promise<SourceEditorHoverResult | null> | null;
		onImplementationLookup?: SourceEditorImplementationLookup;
		onProblemsRequest?: () => void;
		onQuickOpenRequest?: () => void;
		onReferenceLookup?: SourceEditorReferenceLookup;
		onRename?: SourceEditorRename;
		onSaveRequest?: () => void;
		onInlayHintLookup?: SourceEditorInlayHintLookup;
		onSignatureHelpLookup?: SourceEditorSignatureHelpLookup;
		onSymbolsRequest?: () => void;
		onSymbolsChange?: (symbols: SourceSymbol[]) => void;
		onTypeDefinitionLookup?: SourceEditorTypeDefinitionLookup;
		onWorkspaceEditAction?: (action: SourceCodeAction) => void | Promise<void>;
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
		onCodeActionLookup,
		onCommandPaletteRequest,
		onCompletionLookup,
		onDiagnosticsChange,
		onDefinitionLookup,
		onDocumentHighlightLookup,
		onFormatDocument,
		onGoToLineRequest,
		onHoverLookup,
		onImplementationLookup,
		onProblemsRequest,
		onQuickOpenRequest,
		onReferenceLookup,
		onRename,
		onSaveRequest,
		onInlayHintLookup,
		onSignatureHelpLookup,
		onSymbolsRequest,
		onSymbolsChange,
		onTypeDefinitionLookup,
		onWorkspaceEditAction,
	}: Props = $props();

	let host = $state<HTMLDivElement | null>(null);
	let editor = $state<Monaco.editor.IStandaloneCodeEditor | null>(null);
	let monacoApi: typeof Monaco | null = null;
	let contentChangeDisposable: Monaco.IDisposable | null = null;
	let markerChangeDisposable: Monaco.IDisposable | null = null;
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
	let completionProviderDisposable: Monaco.IDisposable | null = null;
	let documentSymbolProviderDisposable: Monaco.IDisposable | null = null;
	let editorActionDisposables: Monaco.IDisposable[] = [];
	let currentPath = "";
	let currentTargetLine: number | null = null;
	let currentTargetLineRequestId = -1;
	let handledIntelligenceCommandId = -1;
	let applyingContent = false;
	let isReady = $state(false);
	let layoutObserver: ResizeObserver | null = null;
	let layoutFrame = 0;
	let externalWorkspaceEditCommandId = "";
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

	function registerSourceDefinitionProvider(monaco: typeof Monaco) {
		definitionProviderDisposable?.dispose();
		definitionProviderDisposable = monaco.languages.registerDefinitionProvider(
			["typescript", "javascript", "csharp"],
			{
				provideDefinition: async (model, position) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return null;

					const targets = await onDefinitionLookup?.(request);
					return (targets ?? []).map((target) => sourceDefinitionTargetToLocation(monaco, target));
				},
			}
		);
	}

	function registerSourceReferenceProvider(monaco: typeof Monaco) {
		referenceProviderDisposable?.dispose();
		referenceProviderDisposable = monaco.languages.registerReferenceProvider(
			["typescript", "javascript", "csharp"],
			{
				provideReferences: async (model, position) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return null;

					const targets = await onReferenceLookup?.(request);
					return (targets ?? []).map((target) => sourceReferenceTargetToLocation(monaco, target));
				},
			}
		);
	}

	function registerSourceDocumentHighlightProvider(monaco: typeof Monaco) {
		documentHighlightProviderDisposable?.dispose();
		documentHighlightProviderDisposable = monaco.languages.registerDocumentHighlightProvider(
			["typescript", "javascript", "csharp"],
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
			["typescript", "javascript", "csharp"],
			{
				provideImplementation: async (model, position) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return null;

					const targets = await onImplementationLookup?.(request);
					return (targets ?? []).map((target) =>
						sourceImplementationTargetToLocation(monaco, target)
					);
				},
			}
		);
	}

	function registerSourceTypeDefinitionProvider(monaco: typeof Monaco) {
		typeDefinitionProviderDisposable?.dispose();
		typeDefinitionProviderDisposable = monaco.languages.registerTypeDefinitionProvider(
			["typescript", "javascript", "csharp"],
			{
				provideTypeDefinition: async (model, position) => {
					const request = lookupRequestForModelPosition(model, position);
					if (!request) return null;

					const targets = await onTypeDefinitionLookup?.(request);
					return (targets ?? []).map((target) =>
						sourceTypeDefinitionTargetToLocation(monaco, target)
					);
				},
			}
		);
	}

	function registerSourceFormattingProvider(monaco: typeof Monaco) {
		formattingProviderDisposable?.dispose();
		formattingProviderDisposable = monaco.languages.registerDocumentFormattingEditProvider(
			["typescript", "javascript", "csharp"],
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
			["typescript", "javascript", "csharp"],
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
			["typescript", "javascript", "csharp"],
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
			["typescript", "javascript", "csharp"],
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
			["typescript", "javascript", "csharp"],
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
			["typescript", "javascript", "csharp"],
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
			["typescript", "javascript", "csharp"],
			{
				provideDocumentSymbols: (model) =>
					extractSourceSymbols(previewForModel(model), model.getValue()).map((symbol) =>
						sourceSymbolToDocumentSymbol(monaco, symbol)
					),
			}
		);
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

	function requestHoverAtCursor() {
		void editor?.getAction("editor.action.showHover")?.run();
	}

	function requestSymbolsAtCursor() {
		const quickOutlineAction = editor?.getAction("editor.action.quickOutline");
		if (quickOutlineAction) {
			void quickOutlineAction.run();
			return;
		}

		onSymbolsRequest?.();
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
			smoothScrolling: true,
			stickyScroll: { enabled: false },
			tabSize: 4,
			theme: sourcePreviewAppearance.theme.id,
			wordWrap: "off",
		});

		externalWorkspaceEditCommandId =
			editor.addCommand(0, (_accessor, action?: SourceCodeAction) => {
				if (action) void onWorkspaceEditAction?.(action);
			}) ?? "";

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
				run: () => requestSymbolsAtCursor(),
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
		completionProviderDisposable?.dispose();
		documentSymbolProviderDisposable?.dispose();
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
