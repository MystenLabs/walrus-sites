# Blocklist API

A CRUD API for managing the blocklist entries.

You can check if a site is included in the blocklist, add, or remove entries.

### Setup

To install dependencies:
```sh
bun install
```

To run:
```sh
bun run dev
```

Environment Variables:
- `REDIS_WRITE_URL`: The URL of the Redis server.
- `BEARER_TOKEN`: The bearer token for authentication.

### Docker
While being in the walrus-sites root directory, run the following commands:
- Build: `docker build -f portal/docker/blocklist_api/Dockerfile -t blocklist_api . --no-cache`
- Run: `docker run --env-file portal/blocklist_api/.env.local -p 3000:3000 blocklist_api`

> Note: The `manual_testing` directory contains a set of scripts that can be used to test the API manually.
The tool used is [bruno](https://www.usebruno.com/), an open-source alternative to Postman.
