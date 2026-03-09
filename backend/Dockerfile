FROM rust:latest

WORKDIR /app

RUN cargo install cargo-watch && cargo install sea-orm-cli

COPY . .

CMD ["cargo", "watch", "-x", "run"]