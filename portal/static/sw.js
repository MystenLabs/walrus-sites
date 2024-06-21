// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Inspired from https://github.com/NekR/self-destroying-sw
self.addEventListener("install", (event) => {
    self.skipWaiting();
});

self.addEventListener("activate", (event) => {
    self.registration
        .unregister()
        .then(() => self.clients.matchAll())
        .then((clients) => {
            clients.forEach((client) => {
                if (client.url && "navigate" in client) {
                    client.navigate(client.url);
                }
            });
        });
});
