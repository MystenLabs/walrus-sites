// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

if ("serviceWorker" in navigator) {
  navigator.serviceWorker
    .getRegistrations()
    .then(function (registrations) {
      registrations.forEach(function (registration) {
        registration.unregister();
      });
    })
    .catch(function (error) {
        console.error("Error unregistering service workers:", error);
    });
}
