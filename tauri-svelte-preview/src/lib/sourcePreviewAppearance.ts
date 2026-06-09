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
			{ token: 'keyword', foreground: 'ff6d91', fontStyle: 'bold' },
			{ token: 'keyword.control', foreground: 'ff6d91', fontStyle: 'bold' },
			{ token: 'type', foreground: '5fd8f2' },
			{ token: 'type.identifier', foreground: '5fd8f2' },
			{ token: 'identifier', foreground: 'e7ecea' },
			{ token: 'namespace', foreground: 'b67af0' },
			{ token: 'string', foreground: 'a7d977' },
			{ token: 'number', foreground: 'e5bb70' },
			{ token: 'comment', foreground: '7c8582', fontStyle: 'italic' },
			{ token: 'operator', foreground: 'f08d7e' },
			{ token: 'delimiter', foreground: '77817e' }
		],
		colors: {
			'editor.background': '#101212',
			'editor.foreground': '#e7ecea',
			'editorLineNumber.foreground': '#52605c',
			'editorLineNumber.activeForeground': '#9fb1ac',
			'editorCursor.foreground': '#5ce2cf',
			'editor.selectionBackground': '#2a696133',
			'editor.inactiveSelectionBackground': '#2a69611f',
			'editor.lineHighlightBackground': '#ffffff08',
			'editorGutter.background': '#101212',
			'editorWidget.background': '#1d2221',
			'editorWidget.border': '#ffffff1a',
			'editorSuggestWidget.background': '#1d2221',
			'scrollbarSlider.background': '#ffffff2b',
			'scrollbarSlider.hoverBackground': '#ffffff42',
			'scrollbarSlider.activeBackground': '#ffffff5a'
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
