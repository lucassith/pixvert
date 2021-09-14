FROM rust:1.55.0-slim as build

RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
 && rm -rf /var/lib/apt/lists/*

RUN USER=root cargo new --bin pixvert_rs
WORKDIR /pixvert_rs

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm ./target/release/deps/pixvert_rs*
RUN cargo build --release

FROM rust:1.55.0-slim

RUN apt-get update && apt-get install -y \
    curl \
 && rm -rf /var/lib/apt/lists/*


RUN mkdir /pixvert_rs
WORKDIR /pixvert_rs

COPY --from=build /pixvert_rs/target/release/pixvert_rs .

CMD ["./pixvert_rs"]
