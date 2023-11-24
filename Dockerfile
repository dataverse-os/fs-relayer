FROM rust:bullseye AS builder-rust

RUN apt update && apt install -y protobuf-compiler libclang-dev

COPY . /build
WORKDIR /build
RUN cargo build --release --no-default-features -p dataverse-file-relayer

FROM debian:bullseye
RUN apt update && apt install -y ca-certificates

COPY --from=builder-rust /build/target/release/dataverse-file-relayer /usr/bin/file-relayer

ENTRYPOINT ["/usr/bin/file-relayer"]