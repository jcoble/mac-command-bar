export const RAIL_UTILITY_REQUEST_EVENT = 'mac-command-bar:rail-utility-request';
export const RAIL_UTILITY_STATE_EVENT = 'mac-command-bar:rail-utility-state';

export type RailUtilityId = 'resources' | 'usage';

export interface RailUtilityAnchor {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface RailUtilityRequest {
  id: RailUtilityId;
  anchor: RailUtilityAnchor;
}

export interface RailUtilityState {
  id: RailUtilityId;
  open: boolean;
}

function isRailUtilityId(value: unknown): value is RailUtilityId {
  return value === 'resources' || value === 'usage';
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

export function railUtilityRequest(
  id: RailUtilityId,
  rect: Pick<DOMRectReadOnly, 'left' | 'top' | 'width' | 'height'>
): RailUtilityRequest {
  return {
    id,
    anchor: {
      left: rect.left,
      top: rect.top,
      width: rect.width,
      height: rect.height
    }
  };
}

export function isRailUtilityRequest(value: unknown): value is RailUtilityRequest {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<RailUtilityRequest>;
  const anchor = candidate.anchor as Partial<RailUtilityAnchor> | undefined;
  return isRailUtilityId(candidate.id)
    && Boolean(anchor)
    && isFiniteNumber(anchor?.left)
    && isFiniteNumber(anchor?.top)
    && isFiniteNumber(anchor?.width)
    && isFiniteNumber(anchor?.height);
}

export function isRailUtilityState(value: unknown): value is RailUtilityState {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<RailUtilityState>;
  return isRailUtilityId(candidate.id) && typeof candidate.open === 'boolean';
}

export function railUtilityAnchorStyle(anchor: RailUtilityAnchor): string {
  return `left: ${anchor.left}px; top: ${anchor.top}px; width: ${anchor.width}px; height: ${anchor.height}px;`;
}
