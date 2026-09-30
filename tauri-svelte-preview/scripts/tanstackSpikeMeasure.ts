/** Linux-only spike receipt: RSS and PSS of the isolated app and descendants. */
import { execFileSync } from 'node:child_process';
import { readFileSync, appendFileSync } from 'node:fs';
const root = Number(process.argv[2]);
const stage = process.argv[3];
const output = process.argv[4];
if (!root || !stage || !output) throw new Error('Usage: <app PID> <stage> <output.jsonl>');
for (let sample = 0; sample < 10; sample++) {
  const rows = execFileSync('ps', ['-eo', 'pid=,ppid=,rss=,comm='], { encoding: 'utf8' })
    .trim().split('\n').map((line) => {
      const [pid, parent, rss, ...name] = line.trim().split(/\s+/);
      return { pid: Number(pid), parent: Number(parent), rssKiB: Number(rss), comm: name.join(' ') };
    });
  const owned = new Set([root]);
  for (let size = 0; size !== owned.size;) {
    size = owned.size;
    for (const row of rows) if (owned.has(row.parent)) owned.add(row.pid);
  }
  const processes = rows.filter((row) => owned.has(row.pid)).map((row) => {
    let pssKiB: number | null = null;
    let cpuTicks: number | null = null;
    try { pssKiB = Number(readFileSync(`/proc/${row.pid}/smaps_rollup`, 'utf8').match(/^Pss:\s+(\d+)/m)?.[1]); }
    catch { /* Process exited between inventory and measurement. */ }
    try {
      const stat = readFileSync(`/proc/${row.pid}/stat`, 'utf8').split(') ')[1].split(' ');
      cpuTicks = Number(stat[11]) + Number(stat[12]);
    } catch { /* Process exited. */ }
    return { ...row, pssKiB, cpuTicks };
  });
  appendFileSync(output, JSON.stringify({ stage, time: new Date().toISOString(), processes }) + '\n');
  await new Promise((resolve) => setTimeout(resolve, 1000));
}
