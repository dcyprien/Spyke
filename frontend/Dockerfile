FROM node:20-alpine

WORKDIR /app

# On installe juste les dépendances
COPY package*.json ./
RUN npm ci

# On copie le reste (même si le docker-compose va superposer ton dossier local par-dessus)
COPY . .

EXPOSE 3001

# Le fameux mode dev avec hot-reload !
CMD ["npm", "run", "dev"]