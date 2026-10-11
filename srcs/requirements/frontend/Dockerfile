# ==========================================
# STAGE 1: The Builder Stage
# ==========================================
FROM node:25-slim AS builder

RUN corepack enable && corepack prepare pnpm@10.28.2 --activate
WORKDIR /app

# copy these 2 files first so there is no new npm install if the file has not changed
# if a change in a file occurs, docker discards every subsequent cached command from scratch
COPY package.json pnpm-lock.yaml ./

# --frozen-lockfile means the build will be consistent with what we developed
# no new patched versions of dependencies
RUN pnpm install --frozen-lockfile

# Copy source code and build the application
COPY . .
RUN pnpm run build

# ==========================================
# STAGE 2: The Production Runner Stage
# ==========================================
FROM node:25-slim AS runner

WORKDIR /app

ENV NODE_ENV=production

# .output is self-contained — nuxt bundles all server deps at build time,
# so we don't need node_modules or package.json in this stage
COPY --from=builder /app/.output ./.output

EXPOSE 3000

CMD ["node", ".output/server/index.mjs"]
