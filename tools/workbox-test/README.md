# Workbox Test

A disposable SSH host on the workbox for remote test scenarios. Proof apps add it
as the machine **"Workbox Test"** (SSH alias `workbox-test`) and use the normal
**Install and connect** flow. It has its own sshd, home, `~/.local/share/assembly`
and loopback port 7777, so test sessions never reach the owner's backend.

- Container `assembly-test`, image `assembly-test`, Ubuntu 24.04, systemd as PID 1
  (the installer needs `systemctl --user`; the user lingers so the backend keeps running).
- sshd is published only on the workbox Tailscale address: `100.86.236.69:2223`.
  Key login only, user `blackcolours`.
- Named volume `assembly-test-home` is mounted at `/home`. It holds the user's home
  (installed backend, sessions database, provider CLIs and logins) and the SSH host
  key, so a rebuild keeps all of it.
- Privileges: `--cap-add SYS_ADMIN --security-opt apparmor=unconfined` lets the
  entrypoint remount the container's own cgroup tree writable for systemd
  (`--cgroupns=private` keeps it scoped to this container). Not `--privileged`.

MacBook `~/.ssh/config`:

```
Host workbox-test
  HostName 100.86.236.69
  Port 2223
  User blackcolours
  IdentityFile ~/.ssh/codex-vps-rental-edi-89-58-18-176
  IdentitiesOnly yes
```

## Commands (run from the repo root on the MacBook)

Build the image (the Dockerfile is sent over SSH; nothing is copied to the workbox):

```
ssh agent-workbox "docker build -t assembly-test --build-arg AUTHORIZED_KEYS='$(cat ~/.ssh/codex-vps-rental-edi-89-58-18-176.pub ~/.ssh/id_ed25519.pub)' -" < tools/workbox-test/Dockerfile
```

Start (first time, or after a rebuild):

```
ssh agent-workbox 'docker run -d --name assembly-test --hostname workbox-test --restart unless-stopped \
  --cgroupns=private --cap-add SYS_ADMIN --security-opt apparmor=unconfined \
  --tmpfs /run --tmpfs /run/lock -v assembly-test-home:/home \
  -p 100.86.236.69:2223:22 assembly-test'
```

- Stop / start again: `ssh agent-workbox docker stop assembly-test` / `docker start assembly-test`.
- Rebuild: build, then `docker rm -f assembly-test` and run the start command. Logins and data stay.
- Reset test data only: `ssh workbox-test 'systemctl --user stop assembly-remote.service && rm -rf ~/.local/share/assembly/sessions.db* && systemctl --user start assembly-remote.service'`.
- Wipe everything (logins too): `docker rm -f assembly-test && docker volume rm assembly-test-home`, then start again.

If the workbox reboots before Tailscale is up, the container cannot bind its port;
run `docker start assembly-test` once Tailscale is connected.

## Provider sign-in (once, by the owner)

`ssh workbox-test`, then:

- Claude: `claude auth login` (open the printed URL, paste the code back).
- Codex: `codex login --device-auth` (open the URL, enter the one-time code).
- Antigravity: `agy`, choose "Google OAuth", finish sign-in, then exit.
  `~/.gemini/antigravity-acp/settings.json` already selects `oauth-personal`.

The CLIs live in `~/.local/bin` inside the container (installed with the official
installers: `claude.ai/install.sh`, `chatgpt.com/codex/install.sh`,
`antigravity.google/cli/install.sh`).
