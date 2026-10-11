FROM node:24-alpine AS builder
WORKDIR /app
RUN npm install -g pnpm@10.28.2  # corepack is no longer bundled from Node 25
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

FROM node:24-alpine AS runner
WORKDIR /app
RUN npm install -g pnpm@10.28.2  # corepack is no longer bundled from Node 25
COPY --from=builder /app/package.json /app/pnpm-lock.yaml ./
RUN pnpm install --prod --frozen-lockfile
COPY --from=builder /app/dist ./dist
# Drizzle migrations might be needed in prod
COPY --from=builder /app/drizzle ./drizzle 

EXPOSE 3000
CMD ["node", "dist/index.js"]
