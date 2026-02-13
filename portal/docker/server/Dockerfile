# use the official Bun image
# see all versions at https://hub.docker.com/r/oven/bun/tags
FROM oven/bun:1.2.5 AS base
WORKDIR /usr/src/app

# install dependencies into temp directory
# this will cache them and speed up future builds
FROM base AS install

RUN apt-get update -y
RUN apt-get install -y ca-certificates

# environment variables, can be overridden by docker build --build-arg <arg>=<value>
ARG ENABLE_ALLOWLIST="false"
ENV ENABLE_ALLOWLIST=${ENABLE_ALLOWLIST}

ARG ENABLE_BLOCKLIST="false"
ENV ENABLE_BLOCKLIST=${ENABLE_BLOCKLIST}

ARG LANDING_PAGE_OID_B36="46f3881sp4r55fc6pcao9t93bieeejl4vr4k2uv8u4wwyx1a93"
ENV LANDING_PAGE_OID_B36=${LANDING_PAGE_OID_B36}

ARG PORTAL_DOMAIN_NAME_LENGTH=""
ENV PORTAL_DOMAIN_NAME_LENGTH=${PORTAL_DOMAIN_NAME_LENGTH}

# Premium RPC URL list is only used when ENABLE_ALLOWLIST is true
ARG PREMIUM_RPC_URL_LIST="https://fullnode.mainnet.sui.io|2|100"
ENV PREMIUM_RPC_URL_LIST=${PREMIUM_RPC_URL_LIST}

# Format: URL|RETRIES|METRIC (lower metric number = higher priority)
ARG RPC_URL_LIST="https://fullnode.mainnet.sui.io|2|100,https://mainnet.suiet.app|1|200"
ENV RPC_URL_LIST=${RPC_URL_LIST}

ARG SUINS_CLIENT_NETWORK="mainnet"
ENV SUINS_CLIENT_NETWORK=${SUINS_CLIENT_NETWORK}

# Format: URL|RETRIES|METRIC (lower metric number = higher priority)
ARG AGGREGATOR_URL_LIST="https://aggregator.walrus-mainnet.walrus.space|2|100"
ENV AGGREGATOR_URL_LIST=${AGGREGATOR_URL_LIST}

ARG SITE_PACKAGE="0x26eb7ee8688da02c5f671679524e379f0b837a12f1d1d799f255b7eea260ad27"
ENV SITE_PACKAGE=${SITE_PACKAGE}

ARG B36_DOMAIN_RESOLUTION_SUPPORT="true"
ENV B36_DOMAIN_RESOLUTION_SUPPORT=${B36_DOMAIN_RESOLUTION_SUPPORT}

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
