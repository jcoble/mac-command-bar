/**
 * The one native browser view has one small logical state record per rail
 * session. These records live only for this shell run; page contents are
 * deliberately reloaded when another session takes ownership of the view.
 */
import type { SessionBrowserWorkspace } from '../sessionWorkspaces.ts';
import type { BrowserInteractionMode, BrowserMarkupCapture } from './browserTypes.ts';
import type { PlacedAnnotationShape } from '../panels/browser/annotationComposite.ts';
import type { BrowserAnnotation } from '../panels/browser/annotationList.ts';

export interface BrowserPanelSessionSnapshot {
  annotations: BrowserAnnotation[];
  strokes: PlacedAnnotationShape[];
  description: string;
  listOpen: boolean;
  tool: BrowserInteractionMode;
  capture: BrowserMarkupCapture | null;
  editingId: string | null;
  expanded: boolean;
}

export interface BrowserSessionSnapshot {
  browser: SessionBrowserWorkspace;
  panel: BrowserPanelSessionSnapshot;
}

const snapshots = new Map<string, BrowserSessionSnapshot>();

function emptySnapshot(): BrowserSessionSnapshot {
  return {
    browser: { url: '', inputUrl: '', activated: false },
    panel: {
      annotations: [],
      strokes: [],
      description: '',
      listOpen: false,
      tool: 'browse',
      capture: null,
      editingId: null,
      expanded: false
    }
  };
}

function copy(snapshot: BrowserSessionSnapshot): BrowserSessionSnapshot {
  return structuredClone(snapshot);
}

export function readBrowserSessionSnapshot(ownedId: string): BrowserSessionSnapshot {
  return copy(snapshots.get(ownedId) ?? emptySnapshot());
}

export function writeBrowserSessionSnapshot(
  ownedId: string,
  update: Partial<BrowserSessionSnapshot>
): void {
  const current = snapshots.get(ownedId) ?? emptySnapshot();
  snapshots.set(ownedId, copy({
    browser: update.browser ?? current.browser,
    panel: update.panel ?? current.panel
  }));
}
