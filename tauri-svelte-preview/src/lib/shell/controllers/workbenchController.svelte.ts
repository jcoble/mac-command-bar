/**
 * Controller for shell tabs, workbench navigation, and frame layout.
 */
import { DEFAULT_CENTER_TAB, DEFAULT_RIGHT_TAB } from '../layout/workbenchTabs';
import type { CenterTabId, RightTabId } from '../workbenchNavigation';
import {
	DEFAULT_DIFF_MODE,
	type DiffMode,
	type SessionWorkspaceSnapshot,
} from '../sessionWorkspaces';
import { gitService } from '../git/gitService';
import { gitCommitFilesService } from '../git/gitCommitFilesService';
import { shellPanels } from '../shellPanels';
import {
	CENTER_MIN_WIDTH,
	DOCK_HEIGHT,
	SESSIONS_MAX_WIDTH,
	SESSIONS_MIN_WIDTH,
	SESSIONS_STRIP_WIDTH,
	SESSIONS_WIDTH,
	TOOLS_MAX_WIDTH,
	TOOLS_MIN_WIDTH,
	type RegionHeightLimits,
	type RegionWidthLimits,
	type ShellRegionId,
} from '../layout/frame';
import { settings, type ProblemsLocation } from '../../settingsStore.svelte';
import type { CenterDockSnapshot } from '../layout/centerDock';
import { writeAssemblySettingFromTauri } from '../../tauriSource';
import {
	captureBrowserState,
	releaseBrowserWorkspace,
	restoreBrowserState,
} from '../browser/browserStore.svelte.ts';

const SESSIONS_COLLAPSED_KEY = 'shell.sessions.collapsed';

export interface FrameControls {
	resetLayout(): void;
	showCenterPanel(id: string): void;
	captureCenterLayout(): CenterDockSnapshot | null;
	restoreCenterLayout(snapshot: CenterDockSnapshot | null | undefined): void;
	setRegionWidth(id: ShellRegionId, width: number, limits?: RegionWidthLimits): void;
	setRegionHeight(id: ShellRegionId, height: number, limits?: RegionHeightLimits): void;
	setDockPresent(present: boolean): void;
	setToolsPresent(present: boolean): void;
	setRegionLimits(id: ShellRegionId, limits: RegionWidthLimits): void;
	regionWidth(id: ShellRegionId): number | null;
}

export class WorkbenchController {
	centerTab = $state<CenterTabId>(DEFAULT_CENTER_TAB);
	rightTab = $state<RightTabId>(DEFAULT_RIGHT_TAB);
	diffMode = $state<DiffMode>(DEFAULT_DIFF_MODE);
	sessionsCollapsed = $state(false);
	rightPanelOpen = $state(true);

	private frameControls: FrameControls | null = null;
	private restoringTabs = false;

	onFrameReady(controls: FrameControls): void {
		this.frameControls = controls;
		controls.setRegionLimits('tools', {
			minimumWidth: TOOLS_MIN_WIDTH,
			maximumWidth: TOOLS_MAX_WIDTH,
		});
		controls.setRegionLimits('center', { minimumWidth: CENTER_MIN_WIDTH });
		controls.setToolsPresent(this.rightPanelOpen);
		this.applyProblemsLocation(settings.panels.problemsLocation);
		if (this.sessionsCollapsed) {
			this.applySessionsWidth(true);
		} else {
			controls.setRegionLimits('sessions', {
				minimumWidth: SESSIONS_MIN_WIDTH,
				maximumWidth: SESSIONS_MAX_WIDTH,
			});
			const restored = controls.regionWidth('sessions');
			if (restored !== null && restored < SESSIONS_MIN_WIDTH) {
				controls.setRegionWidth('sessions', SESSIONS_WIDTH);
			}
		}
	}

	handleCenterPanelShown(id: string): void {
		if (this.isCenterTabId(id)) this.adoptCenterTab(id);
		shellPanels.panelShown(id);
		this.syncGitSurfaceVisibility();
	}

	selectCenterTab(id: CenterTabId): void {
		this.adoptCenterTab(id);
		this.frameControls?.showCenterPanel(id);
		this.syncGitSurfaceVisibility();
	}

	selectRightTab(id: RightTabId): void {
		this.rightTab = id;
		this.setRightPanelOpen(true);
	}

	toggleRightPanel(): void {
		this.setRightPanelOpen(!this.rightPanelOpen);
	}

	private setRightPanelOpen(open: boolean): void {
		this.rightPanelOpen = open;
		this.frameControls?.setToolsPresent(open);
		this.syncRightPanelVisibility();
		this.syncGitSurfaceVisibility();
	}

	private adoptRightTab(id: RightTabId): void {
		this.rightTab = id;
		this.syncRightPanelVisibility();
		this.syncGitSurfaceVisibility();
	}

	adoptCenterTab(id: CenterTabId): void {
		const previous = this.centerTab;
		this.centerTab = id;
		if (previous === 'diff' && id !== 'diff') {
			gitService.clearSelection();
			gitCommitFilesService.clearSelection();
		}
	}

	setDiffMode(mode: DiffMode): void {
		this.diffMode = mode;
	}

	captureSessionState(): Partial<SessionWorkspaceSnapshot> {
		return {
			rightTab: this.rightTab,
			diffMode: this.diffMode,
			browser: captureBrowserState(),
			center: this.frameControls?.captureCenterLayout() ?? undefined,
		};
	}

	/** Remove outgoing lazy surfaces while the next session is being restored. */
	beginSessionSwitch(): void {
		releaseBrowserWorkspace();
		this.adoptRightTab(DEFAULT_RIGHT_TAB);
		this.selectCenterTab(DEFAULT_CENTER_TAB);
	}

	restoreSessionState(snapshot: SessionWorkspaceSnapshot | null): void {
		this.restoringTabs = true;
		try {
			this.diffMode = snapshot?.diffMode ?? DEFAULT_DIFF_MODE;
			restoreBrowserState(snapshot?.browser);
			if (snapshot?.center) this.frameControls?.restoreCenterLayout(snapshot.center);
			const center = snapshot?.center?.activePanelId;
			this.selectCenterTab(this.isCenterTabId(center ?? '') ? center as CenterTabId : DEFAULT_CENTER_TAB);
			this.adoptRightTab(snapshot?.rightTab ?? DEFAULT_RIGHT_TAB);
		} finally {
			this.restoringTabs = false;
		}
		this.syncGitSurfaceVisibility();
	}

	resetLayout(): void {
		this.rightPanelOpen = true;
		this.frameControls?.resetLayout();
		this.applyProblemsLocation(settings.panels.problemsLocation);
		this.syncRightPanelVisibility();
		this.syncGitSurfaceVisibility();
	}

	applyProblemsLocation(location: ProblemsLocation): void {
		const atBottom = location === 'bottom';
		this.frameControls?.setDockPresent(atBottom);
		if (atBottom) {
			this.frameControls?.setRegionHeight('dock', DOCK_HEIGHT, {
				minimumHeight: 96,
				maximumHeight: Number.MAX_SAFE_INTEGER,
			});
		}
	}

	showBottomDock(): void {
		settings.panels.problemsLocation = 'bottom';
		this.applyProblemsLocation('bottom');
	}

	collapseSessions(collapsed: boolean): void {
		this.sessionsCollapsed = collapsed;
		this.applySessionsWidth(collapsed);
		void writeAssemblySettingFromTauri(SESSIONS_COLLAPSED_KEY, collapsed ? 'true' : 'false');
	}

	private applySessionsWidth(collapsed: boolean): void {
		if (!this.frameControls) return;
		if (collapsed) {
			this.frameControls.setRegionLimits('sessions', {
				minimumWidth: SESSIONS_STRIP_WIDTH,
				maximumWidth: SESSIONS_STRIP_WIDTH,
			});
			this.frameControls.setRegionWidth('sessions', SESSIONS_STRIP_WIDTH);
		} else {
			this.frameControls.setRegionLimits('sessions', {
				minimumWidth: SESSIONS_MIN_WIDTH,
				maximumWidth: SESSIONS_MAX_WIDTH,
			});
			this.frameControls.setRegionWidth('sessions', SESSIONS_WIDTH);
		}
	}

	private syncGitSurfaceVisibility(): void {
		if (this.restoringTabs) return;
		const graphVisible = this.centerTab === 'git-history' || (this.rightPanelOpen && this.rightTab === 'source-control');
		shellPanels.sourceControlVisible(graphVisible);
		if (graphVisible) return;
		gitService.releaseHistorySurface();
		gitCommitFilesService.release();
		if (this.centerTab !== 'diff') gitService.clearSelection();
	}

	private syncRightPanelVisibility(): void {
		const visible = this.rightPanelOpen;
		shellPanels.filesVisible(visible && this.rightTab === 'files');
		shellPanels.worktreesVisible(visible && this.rightTab === 'worktrees');
		shellPanels.stacksVisible(visible && this.rightTab === 'run');
		if (visible && this.rightTab === 'browser') shellPanels.panelShown('browser');
	}

	private isCenterTabId(value: string): value is CenterTabId {
		return value === 'session' || value === 'editor' || value === 'diff' || value === 'git-history';
	}
}
