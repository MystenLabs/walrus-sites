# Blocksite Portal Page

A Blocksite portal is a support component in Blocksites that is responsible for
installing the browser-side service workers that perform the website loading.
This directory contains such an implementation of a blocksite portal page.

The primary components of the portal are a (static) index page, which can be
found in `./static`, and the typescript file for the service workers, which can
be found in `./src/sw.ts`. The index page itself is rarely seen by the user, but
it is responsible for initiating the installation of the service worker into the
user's browser. Once installed in the user's browser, the service worker script
handles requests for loading webpages, which involves loading objects from
chain, decoding them, and serving them as responses to the user's requests.

## Running a Local Portal

A local portal can be run from this directory using `pnpm`:

```shell
pnpm install 
pnpm run serve
```

The resulting web server listens on `http://localhost:8080`.
