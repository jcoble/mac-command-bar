export type SourcePaneState = 'expanded' | 'rail' | 'collapsed';

export type SourcePaneSizingConfig = {
  defaultSize: number;
  minSize: number;
  maxSize: number;
  collapseThreshold: number;
  railThreshold?: number;
  railSize?: number;
};

export type SourcePaneStateInput = {
  visible: boolean;
  size: unknown;
};

export type SourcePaneRestoreOptions = {
  previousExpandedSize?: unknown;
};

export type SourcePaneSizingResult = {
  state: SourcePaneState;
  size: number;
  persistedSize: number;
};

type NormalizedSourcePaneSizingConfig = {
  defaultSize: number;
  minSize: number;
  maxSize: number;
  collapseThreshold: number;
  railThreshold?: number;
  railSize?: number;
};

export function clampSourcePaneSize(size: unknown, config: SourcePaneSizingConfig): number {
  const normalizedConfig = normalizeSourcePaneSizingConfig(config);
  return clampToConfigSize(size, normalizedConfig);
}

export function deriveSourcePaneState(
  input: SourcePaneStateInput,
  config: SourcePaneSizingConfig
): SourcePaneState {
  const normalizedConfig = normalizeSourcePaneSizingConfig(config);
  if (!input.visible) return 'collapsed';
  return visibleSourcePaneState(input.size, normalizedConfig);
}

export function resolveSourcePaneSize(
  input: SourcePaneStateInput,
  config: SourcePaneSizingConfig,
  options: SourcePaneRestoreOptions = {}
): SourcePaneSizingResult {
  const normalizedConfig = normalizeSourcePaneSizingConfig(config);

  if (!input.visible) {
    return {
      state: 'collapsed',
      size: 0,
      persistedSize: restoreExpandedSize(input.size, normalizedConfig, options)
    };
  }

  const size = clampToConfigSize(input.size, normalizedConfig);

  return {
    state: visibleSourcePaneState(size, normalizedConfig),
    size,
    persistedSize: size
  };
}

export function finishSourcePanePointerSize(
  rawSize: unknown,
  config: SourcePaneSizingConfig,
  options: SourcePaneRestoreOptions = {}
): SourcePaneSizingResult {
  const normalizedConfig = normalizeSourcePaneSizingConfig(config);
  const numericRawSize = finiteNumber(rawSize, normalizedConfig.defaultSize);

  if (numericRawSize <= normalizedConfig.collapseThreshold) {
    return {
      state: 'collapsed',
      size: 0,
      persistedSize: restoreExpandedSize(numericRawSize, normalizedConfig, options)
    };
  }

  const size = clampToConfigSize(numericRawSize, normalizedConfig);
  const railSize = finishRailSize(size, normalizedConfig);

  if (railSize !== null) {
    return {
      state: 'rail',
      size: railSize,
      persistedSize: railSize
    };
  }

  return {
    state: visibleSourcePaneState(size, normalizedConfig),
    size,
    persistedSize: size
  };
}

export function restoreSourcePaneExpandedSize(
  currentSize: unknown,
  config: SourcePaneSizingConfig,
  options: SourcePaneRestoreOptions = {}
): number {
  return restoreExpandedSize(currentSize, normalizeSourcePaneSizingConfig(config), options);
}

function normalizeSourcePaneSizingConfig(
  config: SourcePaneSizingConfig
): NormalizedSourcePaneSizingConfig {
  const minSize = Math.max(0, Math.round(finiteNumber(config.minSize, 0)));
  const maxSize = Math.max(minSize, Math.round(finiteNumber(config.maxSize, minSize)));
  const defaultSize = clampNumber(finiteNumber(config.defaultSize, minSize), minSize, maxSize);
  const collapseThreshold = Math.round(finiteNumber(config.collapseThreshold, minSize));
  const railThreshold = finiteOptionalNumber(config.railThreshold);
  const normalizedRailThreshold =
    railThreshold !== undefined && railThreshold > collapseThreshold ? Math.round(railThreshold) : undefined;
  const railSize = finiteOptionalNumber(config.railSize);
  const normalizedRailSize =
    normalizedRailThreshold !== undefined && railSize !== undefined
      ? clampNumber(railSize, minSize, maxSize)
      : undefined;

  const normalizedConfig: NormalizedSourcePaneSizingConfig = {
    defaultSize,
    minSize,
    maxSize,
    collapseThreshold
  };

  if (normalizedRailThreshold !== undefined) {
    normalizedConfig.railThreshold = normalizedRailThreshold;
    if (normalizedRailSize !== undefined && normalizedRailSize <= normalizedRailThreshold) {
      normalizedConfig.railSize = normalizedRailSize;
    }
  }

  return normalizedConfig;
}

function visibleSourcePaneState(
  size: unknown,
  config: NormalizedSourcePaneSizingConfig
): SourcePaneState {
  const numericSize = finiteNumber(size, config.defaultSize);
  if (config.railThreshold !== undefined && numericSize <= config.railThreshold) return 'rail';
  return 'expanded';
}

function restoreExpandedSize(
  currentSize: unknown,
  config: NormalizedSourcePaneSizingConfig,
  options: SourcePaneRestoreOptions
): number {
  const previousSize = expandedSize(options.previousExpandedSize, config);
  if (previousSize !== null) return previousSize;

  const currentExpandedSize = expandedSize(currentSize, config);
  return currentExpandedSize ?? config.defaultSize;
}

function expandedSize(size: unknown, config: NormalizedSourcePaneSizingConfig): number | null {
  const clampedSize = clampToConfigSize(size, config);
  return visibleSourcePaneState(clampedSize, config) === 'expanded' ? clampedSize : null;
}

function finishRailSize(size: unknown, config: NormalizedSourcePaneSizingConfig): number | null {
  if (config.railSize === undefined) return null;
  return visibleSourcePaneState(size, config) === 'rail' ? config.railSize : null;
}

function clampToConfigSize(size: unknown, config: NormalizedSourcePaneSizingConfig): number {
  return clampNumber(finiteNumber(size, config.defaultSize), config.minSize, config.maxSize);
}

function clampNumber(size: number, minSize: number, maxSize: number): number {
  return Math.min(maxSize, Math.max(minSize, Math.round(size)));
}

function finiteNumber(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function finiteOptionalNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}
