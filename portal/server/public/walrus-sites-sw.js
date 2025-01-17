// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Inspired by https://github.com/NekR/self-destroying-sw
self.addEventListener("install", (_event) => {
    self.skipWaiting();
});

self.addEventListener("activate", (_event) => {
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
