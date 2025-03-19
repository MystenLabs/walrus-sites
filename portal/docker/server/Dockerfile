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

ARG ENABLE_SENTRY="false"
ENV ENABLE_SENTRY=${ENABLE_SENTRY}
ENV SENTRY_LOG_LEVEL=debug
ENV SENTRY_SUPPRESS_GLOBAL_ERROR_HANDLER_FILE_WARNING=1
ARG SENTRY_AUTH_TOKEN
ENV SENTRY_AUTH_TOKEN=${SENTRY_AUTH_TOKEN}

ARG ENABLE_VERCEL_WEB_ANALYTICS="false"
ENV ENABLE_VERCEL_WEB_ANALYTICS=${ENABLE_VERCEL_WEB_ANALYTICS}

ARG LANDING_PAGE_OID_B36="41qecxqcyzqm8gl0cp2fqd6iq62j0jo5the39nb0bsg39acnib"
ENV LANDING_PAGE_OID_B36=${LANDING_PAGE_OID_B36}

ARG PORTAL_DOMAIN_NAME_LENGTH=""
ENV PORTAL_DOMAIN_NAME_LENGTH=${PORTAL_DOMAIN_NAME_LENGTH}

ARG PREMIUM_RPC_URL_LIST="https://fullnode.testnet.sui.io"
ENV PREMIUM_RPC_URL_LIST=${PREMIUM_RPC_URL_LIST}

ARG RPC_URL_LIST="https://fullnode.testnet.sui.io,https://testnet.suiet.app"
ENV RPC_URL_LIST=${RPC_URL_LIST}

ARG SUINS_CLIENT_NETWORK="testnet"
ENV SUINS_CLIENT_NETWORK=${SUINS_CLIENT_NETWORK}

ARG AGGREGATOR_URL="https://aggregator.walrus-testnet.walrus.space"
ENV AGGREGATOR_URL=${AGGREGATOR_URL}

ARG SITE_PACKAGE="0xc5bebae319fc9d2a9dc858b7484cdbd6ef219decf4662dc81a11dc69bb7a5fa7"
ENV SITE_PACKAGE=${SITE_PACKAGE}

ARG B36_DOMAIN_RESOLUTION_SUPPORT="true"
ENV B36_DOMAIN_RESOLUTION_SUPPORT=${B36_DOMAIN_RESOLUTION_SUPPORT}

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
