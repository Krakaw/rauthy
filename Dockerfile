FROM rust:1.45.1 as builder
WORKDIR /usr/src/nginx-auth
RUN echo "fn main() {}" > dummy.rs
COPY Cargo.toml .
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/nginx-auth /usr/local/bin/nginx-auth
CMD ["nginx-auth"]
