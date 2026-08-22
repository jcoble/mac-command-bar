export type ResourceSampleProcess = {
  pid: number;
  name: string;
  cpuPercent: number;
  rssBytes: number;
};

export type ResourceSampleAppPart = {
  label: string;
  pid: number;
  cpuPercent: number;
  rssBytes: number;
};

/** The last sixty readings of one row, oldest first. */
export type ResourceSampleHistory = {
  cpuPercent: number[];
  rssBytes: number[];
};

export type ResourceDiagnostics = {
  conversations: {
    durableSessionRows: number;
    liveSessionOverlays: number;
    liveRuntimeHandles: number;
    sidecarProcesses: number;
    activeTurns: number;
    pendingPermissions: number;
    pendingInputs: number;
    liveToolCalls: number;
    backgroundWork: number;
    suspendableSessions: number;
    adapterPools?: number;
  };
  database: {
    sessionStoreOpenHandles: number;
  };
  terminals: {
    liveSessions: number;
    transcriptProjections: number;
    userPtys: number;
    agentToolPtys: number;
    runConfigurations: number;
    browserAutomations: number;
    exitedSessionsRetained: number;
  };
  streams: {
    workers: number;
    channels: number;
    queuedFrames: number;
    queuedBytes: number;
  };
  languageServers: {
    runningProcesses: number;
  };
  browser: {
    nativeViews: number;
  };
};

export type ResourceSampleSession = {
  ownedId?: string;
  label: string;
  kind: 'terminal' | 'conversation' | 'other';
  /** The process the app started for this session; every stop request names it. */
  rootPid: number;
  processes: ResourceSampleProcess[];
  history: ResourceSampleHistory;
};

export type ResourceSampleGroup = {
  workspace: string;
  sessions: ResourceSampleSession[];
  history: ResourceSampleHistory;
};

export type ResourceSample = {
  generatedAtMs: number;
  totals: {
    cpuPercent: number;
    rssBytes: number;
    processCount: number;
  };
  diagnostics: ResourceDiagnostics;
  app: {
    parts: ResourceSampleAppPart[];
    history: ResourceSampleHistory;
  };
  groups: ResourceSampleGroup[];
};
