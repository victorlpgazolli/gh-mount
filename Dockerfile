FROM rust:1.66-buster

RUN apt update -y && apt upgrade -y && apt install -y \
    fuse \
    libfuse-dev \
    pkg-config

# install gh cli:
ADD https://github.com/cli/cli/releases/download/v2.21.1/gh_2.21.1_linux_arm64.deb /gh.deb
RUN dpkg -i /gh.deb && rm /gh.deb