# use the official Bun image
# see all versions at https://hub.docker.com/r/oven/bun/tags
FROM oven/bun:1 AS base
WORKDIR /usr/src/app

# environment variables, can be overridden by docker build --build-arg <arg>=<value>
ARG ENABLE_ALLOWLIST="false"
ENV ENABLE_ALLOWLIST=${ENABLE_ALLOWLIST}

ARG ENABLE_BLOCKLIST="false"
ENV ENABLE_BLOCKLIST=${ENABLE_BLOCKLIST}

ARG ENABLE_SENTRY="false"
ENV ENABLE_SENTRY=${ENABLE_SENTRY}

ARG LANDING_PAGE_OID_B36="41qecxqcyzqm8gl0cp2fqd6iq62j0jo5the39nb0bsg39acnib"
ENV LANDING_PAGE_OID_B36=${LANDING_PAGE_OID_B36}

ARG PORTAL_DOMAIN_NAME_LENGTH="21"
ENV PORTAL_DOMAIN_NAME_LENGTH=${PORTAL_DOMAIN_NAME_LENGTH}

ARG PREMIUM_RPC_URL_LIST="https://fullnode.testnet.sui.io"
ENV PREMIUM_RPC_URL_LIST=${PREMIUM_RPC_URL_LIST}

ARG RPC_URL_LIST="https://fullnode.testnet.sui.io,https://testnet.suiet.app"
ENV RPC_URL_LIST=${RPC_URL_LIST}

ARG SUINS_CLIENT_NETWORK="testnet"
ENV SUINS_CLIENT_NETWORK=${SUINS_CLIENT_NETWORK}

# install dependencies into temp directory
# this will cache them and speed up future builds
FROM base AS install
RUN mkdir -p /temp/prod
COPY portal/package.json portal/bun.lock /temp/prod/
COPY portal/common /temp/prod/common
COPY portal/server /temp/prod/server
COPY portal/worker /temp/prod/worker
RUN cd /temp/prod && bun install --frozen-lockfile
RUN cd /temp/prod && bun run build:server

# copy production dependencies and source code into final image
FROM base AS release
COPY --from=install /temp/prod/node_modules node_modules
COPY --from=install /temp/prod/package.json .
COPY --from=install /temp/prod/server/package.json server/package.json
COPY --from=install /temp/prod/server/.next server/.next

# run the app
USER bun
EXPOSE 3000/tcp
ENV NODE_ENV=production
CMD [ "bun", "run", "server:prod" ]
