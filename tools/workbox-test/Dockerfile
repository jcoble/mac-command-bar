# "Workbox Test": a disposable SSH host on the workbox that Assembly installs its
# backend into, so test sessions never reach the owner's backend. See README.md.
FROM ubuntu:24.04
ENV container=docker
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      systemd systemd-sysv dbus dbus-user-session libpam-systemd openssh-server \
      python3 openssl iproute2 ca-certificates curl git sqlite3 \
      libwebkit2gtk-4.1-0 libgtk-3-0t64 \
    && rm -rf /var/lib/apt/lists/* \
    && userdel -r ubuntu \
    && useradd -m -u 1000 -s /bin/bash blackcolours \
    && mkdir -p /var/lib/systemd/linger && touch /var/lib/systemd/linger/blackcolours
ARG AUTHORIZED_KEYS
RUN test -n "$AUTHORIZED_KEYS" \
    && install -d -m 700 -o blackcolours -g blackcolours /home/blackcolours/.ssh \
    && printf '%s\n' "$AUTHORIZED_KEYS" > /home/blackcolours/.ssh/authorized_keys \
    && chown blackcolours:blackcolours /home/blackcolours/.ssh/authorized_keys \
    && chmod 600 /home/blackcolours/.ssh/authorized_keys \
    && install -d -m 700 /home/.ssh-host-keys \
    && ssh-keygen -q -t ed25519 -N '' -f /home/.ssh-host-keys/ssh_host_ed25519_key \
    && rm -f /etc/ssh/ssh_host_* \
    && printf '%s\n' 'HostKey /home/.ssh-host-keys/ssh_host_ed25519_key' 'PasswordAuthentication no' \
         'KbdInteractiveAuthentication no' 'PermitRootLogin no' 'AllowUsers blackcolours' \
         > /etc/ssh/sshd_config.d/workbox-test.conf
STOPSIGNAL SIGRTMIN+3
# Docker mounts the container's own cgroup tree read-only; systemd needs it writable.
CMD ["sh", "-c", "mount -o remount,rw /sys/fs/cgroup && exec /sbin/init"]
