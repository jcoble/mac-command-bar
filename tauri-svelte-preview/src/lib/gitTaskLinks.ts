export type GitTaskID = string;

export type GitTaskLink = {
  id: GitTaskID;
  label: string;
  href?: string;
};

export type GitTaskLinkOptions = {
  baseURL?: string | URL | null;
};

const taskIDPattern = /(?<![A-Za-z0-9_])TSK-\d+(?![A-Za-z0-9_])/gi;
const exactTaskIDPattern = /^\[?\s*(TSK-\d+)\s*\]?$/i;

export function normalizeGitTaskID(input: string | null | undefined): GitTaskID | null {
  const match = exactTaskIDPattern.exec(String(input ?? '').trim());
  return match ? match[1].toUpperCase() : null;
}

export function extractGitTaskIDs(input: string | null | undefined): GitTaskID[] {
  const text = String(input ?? '');
  const seen = new Set<GitTaskID>();
  const taskIDs: GitTaskID[] = [];

  for (const match of text.matchAll(taskIDPattern)) {
    const taskID = match[0].toUpperCase();
    if (seen.has(taskID)) continue;

    seen.add(taskID);
    taskIDs.push(taskID);
  }

  return taskIDs;
}

export function buildGitTaskLink(
  input: string | null | undefined,
  options: GitTaskLinkOptions = {}
): GitTaskLink | null {
  const taskID = normalizeGitTaskID(input) ?? extractGitTaskIDs(input)[0] ?? null;
  if (!taskID) return null;

  const href = buildGitTaskHref(taskID, options.baseURL);
  return href ? { id: taskID, label: taskID, href } : { id: taskID, label: taskID };
}

export function buildGitTaskLinksFromText(
  input: string | null | undefined | readonly (string | null | undefined)[],
  options: GitTaskLinkOptions = {}
): GitTaskLink[] {
  const inputs = Array.isArray(input) ? input : [input];
  const taskIDs = uniqueGitTaskIDs(inputs.flatMap((item) => extractGitTaskIDs(item)));
  return taskIDs.map((taskID) => buildGitTaskLink(taskID, options)).filter(isGitTaskLink);
}

function uniqueGitTaskIDs(taskIDs: GitTaskID[]): GitTaskID[] {
  const seen = new Set<GitTaskID>();
  const uniqueTaskIDs: GitTaskID[] = [];

  for (const taskID of taskIDs) {
    if (seen.has(taskID)) continue;

    seen.add(taskID);
    uniqueTaskIDs.push(taskID);
  }

  return uniqueTaskIDs;
}

function buildGitTaskHref(taskID: GitTaskID, baseURL: string | URL | null | undefined): string | null {
  const rawBaseURL = String(baseURL ?? '').trim();
  if (!rawBaseURL) return null;

  try {
    const url = new URL(rawBaseURL);
    const basePath = url.pathname.endsWith('/') ? url.pathname : `${url.pathname}/`;
    url.pathname = `${basePath}${encodeURIComponent(taskID)}`;
    return url.toString();
  } catch {
    return null;
  }
}

function isGitTaskLink(link: GitTaskLink | null): link is GitTaskLink {
  return link !== null;
}
