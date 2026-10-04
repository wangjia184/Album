FROM everymatrix.jfrog.io/emlab-docker-remote-hub/homebrew/brew:4.6.20

# Build-time HTTPS proxy for downloads (github.com / nodejs.org). Pass via
# --build-arg HTTPS_PROXY=... (BuildKit also auto-injects it from the client
# environment when this ARG is declared).
ARG HTTPS_PROXY

ENV HOME=/home/linuxbrew

RUN sudo rm -f /etc/apt/sources.list.d/github-cli.list /etc/apt/sources.list.d/github-cli.sources \
    && sudo apt-get update && \
    sudo apt-get install -y uidmap iptables iputils-ping net-tools iproute2 gawk attr sudo git nasm build-essential wget procps file curl zip unzip tar pkg-config openssh-server bison autoconf autoconf-archive automake libtool meson protobuf-compiler && \
    sudo apt-get clean && \
    sudo rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

ENV HOMEBREW_NO_AUTO_UPDATE=1
RUN cd "$(brew --repo)" && git remote set-url origin https://mirrors.ustc.edu.cn/brew.git

RUN brew update && \
    brew install docker mise minikube docker-compose && \
    brew cleanup --prune=all 

RUN sudo ln -s ~/.linuxbrew/bin/docker /usr/local/bin/docker && \
    sudo ln -s ~/.linuxbrew/bin/dockerd /usr/local/bin/dockerd && \
    sudo ln -s ~/.linuxbrew/bin/containerd /usr/local/bin/containerd && \
    sudo ln -s ~/.linuxbrew/bin/minikube /usr/local/bin/minikube

RUN sudo groupadd docker && sudo usermod -aG docker $(whoami) && newgrp docker

# Every new interactive shell resolves the mounted docker.sock GID at login and,
# if needed, re-execs itself under that group. Replaces the hardcoded runArgs
# "--group-add 973" (the host GID of /var/run/docker.sock varies per machine).
RUN cat >> /home/linuxbrew/.bashrc <<'EOF'

# ---- Dynamic docker.sock GID (no hardcoded GID) ----
if [ -S /var/run/docker.sock ]; then
    SOCK_GID="$(stat -c '%g' /var/run/docker.sock 2>/dev/null || true)"
    if [ -n "${SOCK_GID:-}" ] && [ "$SOCK_GID" != "0" ]; then
        if getent group "$SOCK_GID" >/dev/null 2>&1; then
            DOCKER_GRP="$(getent group "$SOCK_GID" | cut -d: -f1)"
        else
            DOCKER_GRP="docker-host"
            sudo -n groupadd -g "$SOCK_GID" "$DOCKER_GRP" 2>/dev/null || true
        fi
        if ! id -nG | tr ' ' '\n' | grep -qx "$DOCKER_GRP"; then
            # Supplementary groups are set at process start: re-exec once under the new group
            command -v sg >/dev/null 2>&1 && exec sg "$DOCKER_GRP" -c "exec bash"
        fi
    fi
fi
EOF
 

RUN mise use -g node@24  rust@latest

ENV PATH="$PATH:/home/linuxbrew/.local/share/mise/shims"
RUN cargo install cargo-vcpkg  &&  rustup component add rust-src

RUN npm install -g npm@latest && \
    npm config set prefix "$HOME/.local" --location=user && \
    npm install -g opencode-ai

# npm global binaries go to ~/.local/bin (prefix override above) — expose for non-interactive shells
ENV PATH="$PATH:/home/linuxbrew/.local/bin"


RUN mkdir -p "$HOME/.docker/cli-plugins" && \
    ln -sfn "$(brew --prefix)/lib/docker/cli-plugins/docker-compose" \
            "$HOME/.docker/cli-plugins/docker-compose"

RUN node /home/linuxbrew/.local/lib/node_modules/opencode-ai/postinstall.mjs

ADD entrypoint.sh /entrypoint.sh
RUN sudo chmod +x /entrypoint.sh

# 运行时以 root 执行 entrypoint (直读 /root/authorized_keys, 免 sudo);
# VS Code 会话用户仍由 devcontainer.json 的 remoteUser=linuxbrew 决定。
USER root
ENTRYPOINT ["/entrypoint.sh"]

