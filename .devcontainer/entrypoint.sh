#!/bin/bash
# 本脚本以 root 运行（Dockerfile: USER root + ENTRYPOINT）。
# 注意: $HOME=/home/linuxbrew (镜像 ENV), 涉及用户文件的属主必须显式给 linuxbrew,
#       否则 sshd 以 linuxbrew 身份读不到 authorized_keys。

# ---------------------------------------------------------------------------
# Docker socket access (docker-from-docker in a Dev Container).
# 将 $USER 加入与宿主机 docker.sock 同 GID 的组 (组名对齐, 见原注释)。
# ---------------------------------------------------------------------------
if [ -S /var/run/docker.sock ]; then
    SOCK_GID="$(stat -c '%g' /var/run/docker.sock 2>/dev/null || true)"
    if [ -n "${SOCK_GID:-}" ] && [ "$SOCK_GID" != "0" ]; then
        if getent group "$SOCK_GID" >/dev/null 2>&1; then
            DOCKER_GRP="$(getent group "$SOCK_GID" | cut -d: -f1)"
        else
            DOCKER_GRP="docker-host"
            groupadd -g "$SOCK_GID" "$DOCKER_GRP" 2>/dev/null || true
        fi
        usermod -aG "$DOCKER_GRP" linuxbrew
    fi
fi

# 安装容器 SSH 公钥 (root 可直读 /root/authorized_keys, 无需 wrapper)
if [ -f /root/authorized_keys ]; then
    mkdir -p "$HOME/.ssh"
    cp -f /root/authorized_keys "$HOME/.ssh/authorized_keys"
    chown linuxbrew:linuxbrew "$HOME/.ssh" "$HOME/.ssh/authorized_keys"
    chmod 700 "$HOME/.ssh"
    chmod 600 "$HOME/.ssh/authorized_keys"
fi

printf '\nexport PATH="$PATH:/home/linuxbrew/.local/share/mise/shims:/home/linuxbrew/.local/bin"' >> "$HOME/.bashrc"
printf "\nexport HTTP_PROXY=$HTTP_PROXY" >> "$HOME/.bashrc"
printf "\nexport HTTPS_PROXY=$HTTPS_PROXY" >> "$HOME/.bashrc"
printf "\nexport NO_PROXY=$NO_PROXY" >> "$HOME/.bashrc"
printf "\nexport NEW_API_KEY=$MY_LLM_TOKEN" >> "$HOME/.bashrc"

# (removed) sudo chown -R $HOME — 镜像内属主已是 linuxbrew, 且每次启动 copy-up 2.9G 耗时 1-3 分钟, 阻塞 sshd

if [ -n "$SSH_PORT" ]; then
    sed -i 's/#PubkeyAuthentication yes/PubkeyAuthentication yes/' /etc/ssh/sshd_config
    sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config
    sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config
    sed -i '/^#AuthorizedKeysFile/c\AuthorizedKeysFile	.ssh\/authorized_keys' /etc/ssh/sshd_config

    mkdir -p /run/sshd
    chmod 0755 /run/sshd

    # sshd 必须 root 运行 (host key / PAM / setuid 到登录用户), 认证后子进程降为 linuxbrew
    /usr/sbin/sshd -D -E /var/log/sshd.log -p "$SSH_PORT"
fi