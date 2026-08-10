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

export type ResourceSampleSession = {
  ownedId?: string;
  label: string;
  kind: 'terminal' | 'conversation' | 'other';
  processes: ResourceSampleProcess[];
};

export type ResourceSampleGroup = {
  workspace: string;
  sessions: ResourceSampleSession[];
};

export type ResourceSample = {
  generatedAtMs: number;
  totals: {
    cpuPercent: number;
    rssBytes: number;
    processCount: number;
  };
  app: {
    parts: ResourceSampleAppPart[];
  };
  groups: ResourceSampleGroup[];
};
