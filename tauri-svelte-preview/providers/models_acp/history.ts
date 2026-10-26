// Each session's chosen model and messages, one JSON file per session in the sidecar's data folder,
// so a fresh process can answer `session/resume` after the app stops the idle one.
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import type { ModelMessage } from '@tanstack/ai'

export type SavedSession = { model: string, messages: Array<ModelMessage> }

// Session ids are UUIDs we made; anything else can't name a file of ours.
const SESSION_ID = /^[0-9a-f-]{36}$/

export async function loadHistory(dataDir: string, sessionId: string): Promise<SavedSession | undefined> {
  if (!SESSION_ID.test(sessionId)) return undefined
  try {
    return JSON.parse(await readFile(join(dataDir, `${sessionId}.json`), 'utf8'))
  } catch (error) {
    if ((error as { code?: string }).code === 'ENOENT') return undefined
    throw error
  }
}

export async function saveHistory(dataDir: string, sessionId: string, saved: SavedSession): Promise<void> {
  await mkdir(dataDir, { recursive: true })
  await writeFile(join(dataDir, `${sessionId}.json`), JSON.stringify(saved))
}
