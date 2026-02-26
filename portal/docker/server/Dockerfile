# use the official Bun image
# see all versions at https://hub.docker.com/r/oven/bun/tags
FROM oven/bun:1.2.5 AS base
WORKDIR /usr/src/app

# install dependencies into temp directory
# this will cache them and speed up future builds
FROM base AS install

RUN apt-get update -y
RUN apt-get install -y ca-certificates

# Portal configuration is loaded from a YAML file at runtime (mounted via ConfigMap).
# Set PORTAL_CONFIG to the path of the YAML file, or place portal-config.yaml in the CWD.
# Env vars can still be used as overrides — see portal-config.*.yaml.example for the schema.
ARG PORTAL_CONFIG=""
ENV PORTAL_CONFIG=${PORTAL_CONFIG}

ARG PROMETHEUS_EXPORTER_PORT="9184"
ENV PROMETHEUS_EXPORTER_PORT=${PROMETHEUS_EXPORTER_PORT}

RUN mkdir -p /temp/prod
COPY portal/package.json portal/bun.lock /temp/prod/
COPY portal/common /temp/prod/common
COPY portal/server /temp/prod/server
COPY portal/worker /temp/prod/worker
RUN cd /temp/prod && bun install --frozen-lockfile

# debug image target
FROM base as debug
RUN apt update && apt install -y curl netcat-traditional
COPY --from=install /temp/prod/node_modules node_modules
COPY --from=install /temp/prod/package.json .
COPY --from=install /temp/prod/common ./common
COPY --from=install /temp/prod/server ./server

# run the app
USER bun
EXPOSE 3000/tcp
ENV NODE_ENV=development
CMD [ "bun", "run", "server" ]

# copy production dependencies and source code into final image
FROM base AS release
COPY --from=install /temp/prod/node_modules node_modules
COPY --from=install /temp/prod/package.json .
COPY --from=install /temp/prod/common ./common
COPY --from=install /temp/prod/server ./server

# run the app
USER bun
EXPOSE 3000/tcp
ENV NODE_ENV=production
CMD [ "bun", "run", "server" ]
