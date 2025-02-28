# use the official Bun image
# see all versions at https://hub.docker.com/r/oven/bun/tags
FROM oven/bun:1.2.1 AS base
WORKDIR /usr/src/app

FROM base AS install
ARG REDIS_URL=""
ENV REDIS_URL=${REDIS_URL}

ARG BEARER_TOKEN=""
ENV BEARER_TOKEN=${BEARER_TOKEN}

RUN mkdir -p /temp/prod
COPY portal/blocklist_api/package.json portal/blocklist_api/bun.lock /temp/prod/
COPY portal/blocklist_api/ /temp/prod/blocklist_api
RUN cd /temp/prod && bun install --frozen-lockfile

FROM base AS debug
RUN apt update && apt install -y curl netcat-traditional
COPY --from=install /temp/prod/node_modules node_modules
COPY --from=install /temp/prod/package.json .
COPY --from=install /temp/prod/node_modules node_modules
COPY portal/blocklist_api/ .

# run the app
USER bun
EXPOSE 3000/tcp
ENV NODE_ENV=production
ENTRYPOINT [ "bun", "run", "start"]
