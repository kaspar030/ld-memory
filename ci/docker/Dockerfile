#
# This Dockerfile is used to provide a binary-only container that other
# Dockerfiles can `COPY` from.
#
# Use like this in your project's Dockerfile:
#
#    `COPY --from=kaspar030/ldmemory:<version>` /ld-memory /usr/bin
#
# `version`: every semver combination should work (0, 0.1, 0.1.17, ...)
#
ARG DEBIAN_FROM=ubuntu:latest
FROM ${DEBIAN_FROM} AS builder

LABEL maintainer="Kaspar Schleiser <kaspar@schleiser.de>"

ENV DEBIAN_FRONTEND noninteractive

# install rust if not available.
RUN if test -z "$(command -v cargo)"; then apt-get update && apt-get -y install build-essential curl ; fi
RUN if test -z "$(command -v cargo)"; then curl https://sh.rustup.rs -sSf | sh -s -- -y ; fi

# this works on both debian and ubuntu
ENV PATH="/root/.cargo/bin:${PATH}"

# set our target
ARG TARGET=x86_64-unknown-linux-musl

RUN rustup target add ${TARGET}

RUN mkdir /src
COPY / /src

ARG SOURCES="--path /src/tools/ld-memory"

# build laze
RUN cargo install --target ${TARGET} ${SOURCES}
RUN cp $(command -v ld-memory) /
RUN strip -s /ld-memory

FROM scratch

COPY --from=builder /ld-memory /
