# 1) Build step
FROM rust:1.81-bullseye as builder

WORKDIR /usr/src/blog

# RUN apt-get update && apt-get install -y curl
# RUN curl --ipv4 --fail http://www.google.com
# RUN curl --ipv6 --fail http://www.google.com

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --locked --release

# 2) Run step
FROM debian:bullseye-slim

WORKDIR /usr/src/blog

RUN apt-get update && apt-get install -y libssl-dev

COPY --from=builder /usr/src/blog/target/release/JetBrainsBlogTask .

RUN mkdir -p /data

CMD ["./JetBrainsBlogTask", "--db-file", "/data/blog.db"]
