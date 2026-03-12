# --- Étape 1 : La Forge (Build) ---
FROM rust:latest as builder

WORKDIR /app

# On installe les outils nécessaires à la compilation de Sea-ORM et SSL
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# On copie tout le dossier backend
COPY . .

# On compile le binaire final en mode "Release" (super optimisé)
# C'est ici que ça va prendre du temps, mais Render attendra la fin du build
RUN cargo build --release

# --- Étape 2 : Le Messager (Runtime) ---
FROM debian:bookworm-slim

WORKDIR /app

# On installe juste le minimum pour faire tourner le binaire et parler à Neon
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# IMPORTANT : On récupère le binaire compilé à l'étape 1
# Remplace "NOM_DE_TON_PROJET" par le 'name' qui est dans ton Cargo.toml
COPY --from=builder /app/target/release/NOM_DE_TON_PROJET /app/server

EXPOSE 3000

# On lance le binaire directement. Démarrage instantané !
CMD ["./server"]
