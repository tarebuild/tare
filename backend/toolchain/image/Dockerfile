# Build context: backend/. Pins are recorded in image/locks.json.
FROM --platform=linux/amd64 rust:1.99.0-slim-bookworm@sha256:ba81bc3eaa4422af576c0262515d96b0111a628a6ccc2c86557cf55c9a4bbee0

RUN rm -f /etc/apt/sources.list /etc/apt/sources.list.d/debian.sources && \
    printf '%s\n' \
      'deb [check-valid-until=no] https://snapshot.debian.org/archive/debian/20261101T000000Z bookworm main' \
      'deb [check-valid-until=no] https://snapshot.debian.org/archive/debian/20261101T000000Z bookworm-updates main' \
      'deb [check-valid-until=no] https://snapshot.debian.org/archive/debian-security/20261101T000000Z bookworm-security main' \
      > /etc/apt/sources.list && \
    apt-get -o Acquire::Check-Valid-Until=false update && \
    apt-get install -y --no-install-recommends build-essential clang libclang-dev \
      libssl-dev pkg-config curl bzip2 git ca-certificates cmake zlib1g-dev python3 python3-jsonschema && \
    rm -rf /var/lib/apt/lists/*

RUN curl --fail --location --retry 3 \
      https://github.com/anza-xyz/agave/releases/download/v4.4.0/solana-release-x86_64-unknown-linux-gnu.tar.bz2 \
      --output /tmp/agave.tar.bz2 && \
    echo '3363f4728fe786ac7036e342a20df928aa2e3e43f807fbdce67814840538c574  /tmp/agave.tar.bz2' | sha256sum --check --strict && \
    tar -xjf /tmp/agave.tar.bz2 -C /opt && rm /tmp/agave.tar.bz2
RUN mkdir -p /root/.cache/solana/v1.57/platform-tools && \
    curl --fail --location --retry 3 \
      https://github.com/anza-xyz/platform-tools/releases/download/v1.57/platform-tools-linux-x86_64.tar.bz2 \
      --output /tmp/platform-tools.tar.bz2 && \
    echo 'b0f7af104adf726fff2a6a09ea2eb2f2d2965c92295f4d7388c08d140e0c2b00  /tmp/platform-tools.tar.bz2' | sha256sum --check --strict && \
    tar -xjf /tmp/platform-tools.tar.bz2 -C /root/.cache/solana/v1.57/platform-tools && \
    rm /tmp/platform-tools.tar.bz2

ENV PATH="/opt/solana-release/bin:${PATH}" \
    TARE_HOME=/opt/tare \
    TARE_REGISTRY_SOURCE=/opt/tare-registry \
    TARE_IN_CONTAINER=1 \
    TARE_BUILD_IMAGE=tare-toolchain:0.7.0
WORKDIR /opt/tare
COPY toolchain/Cargo.toml toolchain/Cargo.lock toolchain/LICENSE toolchain/NOTICE ./
COPY toolchain/crates ./crates
COPY toolchain/image/locks.json ./image/locks.json
COPY registry /opt/tare-registry
COPY bench/loader-check /opt/bench/loader-check
RUN cargo build --locked --release --manifest-path /opt/bench/loader-check/Cargo.toml --target-dir /opt/tare/target/native-versions && \
    cargo build --locked --release -p tare && \
    install -m 0755 target/release/tare /usr/local/bin/tare && \
    cargo-build-sbf --version && \
    tare --version
WORKDIR /workspace
ENTRYPOINT ["tare"]
CMD ["--version"]
