FROM rust:1.66-buster

RUN apt update -y && apt upgrade -y && apt install -y \
    fuse \
    libfuse-dev \
    pkg-config
