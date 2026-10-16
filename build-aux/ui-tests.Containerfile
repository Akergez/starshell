# Runtime for build-aux/screenshots.sh --headless: sway on wlroots' headless
# backend and Mesa's software Vulkan. The shell is built on the host and
# mounted in, so this matches the host's distribution to share its libraries.
ARG FEDORA=44
FROM registry.fedoraproject.org/fedora:${FEDORA}
RUN dnf install -y --setopt=install_weak_deps=False \
        sway grim mesa-vulkan-drivers vulkan-loader \
        libxkbcommon libxkbcommon-x11 libxcb pam wayland-devel fontconfig \
        google-noto-sans-fonts \
        dbus-daemon dbus-tools findutils procps-ng \
    && dnf clean all
# What the shell links against, and the launcher backend with what its
# calculator and file plugins run.
RUN dnf install -y --setopt=install_weak_deps=False pipewire-libs pulseaudio-libs \
        pop-launcher qalculate fd-find \
    && dnf clean all
# Fedora ships sway with cap_sys_nice; a rootless container cannot grant it,
# so exec fails with EPERM. Headless sway needs no realtime priority.
RUN dnf install -y --setopt=install_weak_deps=False libcap \
    && setcap -r /usr/bin/sway \
    && dnf clean all
# sway refuses to start as root.
RUN useradd --create-home ui
USER ui
