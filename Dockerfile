FROM rust:1.87 AS builder
WORKDIR /usr/src/shame_bot
COPY . .
RUN cargo install --path .

FROM debian:bullseye
COPY --from=builder /usr/local/cargo/bin/shame_bot /usr/local/bin/shame_bot
CMD ["shame_bot"]