FROM rust:1.87 AS builder
WORKDIR /usr/src/shame_bot
COPY . .
ARG DATABASE_URL $DATABASE_URL
# RUN cargo install sqlx-cli
# RUN cargo sqlx prepare
RUN cargo install --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/shame_bot /usr/local/bin/shame_bot
CMD ["shame_bot"]