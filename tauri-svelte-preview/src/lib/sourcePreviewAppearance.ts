type ThemeDefinition = {
	id: string;
	base: 'vs' | 'vs-dark' | 'hc-black' | 'hc-light';
	inherit: boolean;
	rules: Array<{
		token: string;
		foreground?: string;
		background?: string;
		fontStyle?: string;
	}>;
	colors: Record<string, string>;
};

export type SourcePreviewAppearance = {
	fontFamily: string;
	fontLigatures: boolean | string;
	fontSize: number;
	letterSpacing: number;
	lineHeight: number;
	theme: ThemeDefinition;
};

export const sourcePreviewAppearance: SourcePreviewAppearance = {
	fontFamily: '"Google Sans Mono", "SF Mono", ui-monospace, Menlo, Monaco, Consolas, monospace',
	fontLigatures: false,
	fontSize: 13,
	letterSpacing: 0,
	lineHeight: 21,
	theme: {
		id: 'houston',
		base: 'vs-dark',
		inherit: true,
		rules: [
			{ token: 'comment', foreground: '8f9199', fontStyle: 'italic' },
			{ token: 'constant', foreground: '54b9ff' },
			{ token: 'delimiter', foreground: 'eef0f9' },
			{ token: 'function', foreground: '00daef' },
			{ token: 'identifier', foreground: '4bf3c8' },
			{ token: 'keyword', foreground: '54b9ff' },
			{ token: 'keyword.control', foreground: '54b9ff' },
			{ token: 'keyword.operator', foreground: 'eef0f9' },
			{ token: 'namespace', foreground: 'acafff' },
			{ token: 'number', foreground: 'ffd493' },
			{ token: 'operator', foreground: 'eef0f9' },
			{ token: 'regexp', foreground: 'eef0f9' },
			{ token: 'string', foreground: 'ffd493' },
			{ token: 'type', foreground: 'acafff' },
			{ token: 'type.identifier', foreground: 'acafff' },
			{ token: 'variable', foreground: '4bf3c8' },
			{ token: 'variable.predefined', foreground: 'acafff' }
		],
		colors: {
			'editor.background': '#17191e',
			'editor.findMatchBackground': '#515c6a',
			'editor.findMatchBorder': '#74879f',
			'editor.findMatchHighlightBackground': '#ea5c0055',
			'editor.findMatchHighlightBorder': '#ffffff00',
			'editor.findRangeHighlightBackground': '#23262d',
			'editor.findRangeHighlightBorder': '#b2434300',
			'editor.foldBackground': '#ad5dca26',
			'editor.foreground': '#eef0f9',
			'editor.hoverHighlightBackground': '#5495d740',
			'editor.inactiveSelectionBackground': '#2a2d34',
			'editor.lineHighlightBackground': '#23262d',
			'editor.lineHighlightBorder': '#ffffff00',
			'editor.rangeHighlightBackground': '#ffffff0b',
			'editor.rangeHighlightBorder': '#ffffff00',
			'editor.selectionBackground': '#ad5dca44',
			'editor.selectionHighlightBackground': '#add6ff34',
			'editor.selectionHighlightBorder': '#495f77',
			'editor.wordHighlightBackground': '#494949b8',
			'editor.wordHighlightStrongBackground': '#004972b8',
			'editorBracketMatch.background': '#545864',
			'editorBracketMatch.border': '#ffffff00',
			'editorCursor.background': '#000000',
			'editorCursor.foreground': '#aeafad',
			'editorGutter.addedBackground': '#4bf3c8',
			'editorGutter.background': '#17191e',
			'editorGutter.commentRangeForeground': '#545864',
			'editorGutter.deletedBackground': '#f06788',
			'editorGutter.foldingControlForeground': '#545864',
			'editorGutter.modifiedBackground': '#54b9ff',
			'editorLineNumber.activeForeground': '#858b98',
			'editorLineNumber.foreground': '#545864',
			'editorSuggestWidget.background': '#252526',
			'editorSuggestWidget.border': '#454545',
			'editorSuggestWidget.foreground': '#d4d4d4',
			'editorSuggestWidget.highlightForeground': '#0097fb',
			'editorSuggestWidget.selectedBackground': '#062f4a',
			'editorWidget.background': '#343841',
			'editorWidget.foreground': '#ffffff',
			'editorWidget.resizeBorder': '#cc75f4',
			'minimap.background': '#17191e',
			'scrollbarSlider.activeBackground': '#54b9ff66',
			'scrollbarSlider.background': '#54586466',
			'scrollbarSlider.hoverBackground': '#545864B3'
		}
	}
};

export const sourcePreviewAppearanceKey = JSON.stringify({
	fontFamily: sourcePreviewAppearance.fontFamily,
	fontLigatures: sourcePreviewAppearance.fontLigatures,
	fontSize: sourcePreviewAppearance.fontSize,
	letterSpacing: sourcePreviewAppearance.letterSpacing,
	lineHeight: sourcePreviewAppearance.lineHeight,
	theme: sourcePreviewAppearance.theme
});
