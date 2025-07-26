FROM rust:1.87 AS builder
WORKDIR /usr/src/shame_bot
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/shame_bot /usr/local/bin/shame_bot
ENV RUST_LOG='serenity::gateway::shard=off,serenity=warn,shame_bot=info'
CMD ["shame_bot"]