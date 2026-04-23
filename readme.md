# Chat App - T-DEV-600

Bienvenue dans le dépôt du projet **T-DEV-600**. Il s'agit d'une application de messagerie de type Discord/Slack comprenant des serveurs, des canaux, des messages directs, des réactions, et avec un fonctionnement en temps réel. 

Le système complet inclut un backend performant en Rust, un frontend web moderne via Next.js, et des applications de bureau (Windows/Mac/Linux) générées via Tauri.

## Stack Technique

### Backend 
- **Langage** : Rust
- **Framework Web** : Axum / Actix (avec WebSockets pour le temps réel)
- **ORM** : SeaORM
- **Base de données** : PostgreSQL
- **Authentification** : JWT

### Frontend & Desktop 
- **Langage / Framework** : TypeScript, React, Next.js
- **Styling** : Tailwind CSS
- **Desktop Wrapper** : Tauri (permettant la compilation d'applications natives)
- **CI/CD** : GitHub Actions pour les tests et la build multi-plateformes

### Infrastructure
- **Conteneurisation** : Docker & Docker Compose
- **Outils** : Makefile pour l'automatisation

---

## Architecture du Projet

```text
T-DEV-600/
├── backend/          # API REST et WebSocket en Rust
│   ├── migration/    # Migrations SeaORM (schémas PostgreSQL)
│   ├── src/          # Code source de l'API (Architecture Domain/Application/Infrastructure)
│   └── tests/        # Tests unitaires et d'intégration
├── frontend/         # Application Web Next.js locale
│   ├── app/          # Vues et Layouts
│   ├── components/   # Composants React isolés (Bandes, chat, formulaires)
│   └── src-tauri/    # Configuration et point d'entrée pour l'application Desktop
└── docker-compose.yml # Fichier de déploiement des services
```

---

## Démarrage Rapide (Développement)

L'utilisation de Docker et Docker Compose est recommandée pour lancer l'environnement complet de développement (Base de données + Backend).

### Prérequis
- [Docker](https://www.docker.com/) & Docker Compose
- [Node.js](https://nodejs.org/) & NPM/Yarn (pour le frontend local)
- [Rust](https://rustup.rs/) (si vous compilez le backend/tauri en local)
- (Optionnel) Un fichier `.env` configuré avec vos identifiants PostgreSQL et la clé `JWT_SECRET`.

### Lancer les services backend avec Docker

```bash
# Lance PostgreSQL et l'image de build backend via le docker-compose
docker-compose up --build -d
```
*Le backend et la base de données tourneront en arrière-plan.*

### Lancer le Frontend (Web local)

```bash
cd frontend
npm install
npm run dev
```

---

## Application de Bureau (Tauri)

Grâce à Tauri, le frontend Next.js peut être compilé sous forme d'application native très légère.

1. Installez les dépendances du système nécessaires pour Tauri (voir [TAURI_SETUP.md](./TAURI_SETUP.md)).
2. Lancez l'environnement Tauri de dev ou buildez l'application native :

```bash
cd frontend

# Mode développement avec auto-reload pour l'app desktop
npm run tauri:dev

# Compiler et générer un exécutable de production 
npm run tauri:build
```

---

## Tests

Les tests backend (intégration, services divers comme l'authentification et les messages) et la couverture sont mis en place de manière rigoureuse en Rust.

```bash
cd backend
cargo test
```

## Licence & Contributeurs
Projet réalisé dans le cadre du module **T-DEV-600** (Epitech).
