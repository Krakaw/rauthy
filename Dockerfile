FROM rustlang/rust:nightly-stretch-slim as builder
WORKDIR /usr/src/rauthy
COPY . .
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/rauthy/target/release/rauthy /usr/local/bin/rauthy
EXPOSE 3031
CMD ["rauthy"]
