/**
 * The project registry as the front end holds it: a copy of the SQLite
 * `projects` table, read when a screen that needs it opens. It persists
 * nothing itself and never polls.
 */

import { addProjectFromTauri, listProjectsFromTauri, type ProjectRecord } from '../../tauriSource.ts';

export const projectRegistry = $state<{ projects: ProjectRecord[]; hydrated: boolean; error: string }>({
  projects: [],
  hydrated: false,
  error: ''
});

/** Reads the registry from SQLite once per call. */
export async function hydrateProjects(): Promise<void> {
  try {
    projectRegistry.projects = await listProjectsFromTauri();
    projectRegistry.error = '';
  } catch (error) {
    projectRegistry.error = error instanceof Error ? error.message : String(error);
  } finally {
    projectRegistry.hydrated = true;
  }
}

/** Inspects and registers a folder; adding the same folder again returns the same project. */
export async function addProject(machine: string, path: string): Promise<ProjectRecord> {
  const project = await addProjectFromTauri(machine, path);
  if (!projectRegistry.projects.some((existing) => existing.id === project.id)) {
    projectRegistry.projects = [...projectRegistry.projects, project];
  }
  return project;
}

export function projectById(id: string | null): ProjectRecord | null {
  return projectRegistry.projects.find((project) => project.id === id) ?? null;
}
