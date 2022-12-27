FROM rust:1.66-buster

RUN apt update -y && apt upgrade -y && apt install -y \
    fuse \
    libfuse-dev \
    pkg-config \
    wget

ENV GH_CLI_VERSION=2.21.1
RUN export ARCH=$(dpkg --print-architecture);echo $ARCH;

# install gh cli:
RUN wget https://github.com/cli/cli/releases/download/v$(echo $GH_CLI_VERSION)/gh_$(echo $GH_CLI_VERSION)_linux_$(echo $ARCH).deb -o /gh.deb
RUN dpkg -i /gh.deb && rm /gh*.deb