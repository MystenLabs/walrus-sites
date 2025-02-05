# use the official Bun image
# see all versions at https://hub.docker.com/r/oven/bun/tags
FROM oven/bun:1 AS base
WORKDIR /usr/src/app

# environment variables, can be overridden by docker build --build-arg <arg>=<value>
ARG LANDING_PAGE_OID_B36="41qecxqcyzqm8gl0cp2fqd6iq62j0jo5thr39nb0bsg39acnib"
ENV LANDING_PAGE_OID_B36=${LANDING_PAGE_OID_B36}

ARG PORTAL_DOMAIN_NAME_LENGTH="21"
ENV PORTAL_DOMAIN_NAME_LENGTH=${PORTAL_DOMAIN_NAME_LENGTH}

# install dependencies into temp directory
# this will cache them and speed up future builds
FROM base AS install
RUN mkdir -p /temp/prod
COPY package.json bun.lock /temp/prod/
COPY common /temp/prod/common
COPY server /temp/prod/server
COPY worker /temp/prod/worker
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
