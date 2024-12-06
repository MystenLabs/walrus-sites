// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

navigator.serviceWorker
.register("/walrus-sites-portal-register-sw.js")
.then((registration) =>
    console.log(
    "Service Worker registration successful with scope: ",
    registration.scope,
    ),
)
.catch((err) => console.log("Service Worker registration failed: ", err));
