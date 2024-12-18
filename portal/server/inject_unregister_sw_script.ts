// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
* Adds a script to the response to unregister the service worker.
*
* This is a temporary workaround used to transition from serving the server-portal
* through the `walrus.site` domain.
* To prevent the service worker from intercepting with the portal's response,
* this script unregisters the old service worker.
*
* @param response - Any response returned by the portal.
* @returns a new response with the script injected.
*/
export async function inject_unregister_service_worker_script(response: Response): Promise<Response> {
    let responseBody = await response.text();
    const script = `
        <script>
            if ('serviceWorker' in navigator) {
                console.log('Unregistering the walrus sites service-worker!');
                navigator.serviceWorker.getRegistrations().then(registrations => {
                    registrations.forEach(registration => {
                        registration.unregister();
                        console.log('Service worker successfully unregistered.');
                    });
                });
            }
        </script>
    `;

    // Inject the script before the closing body tag.
    responseBody = responseBody.replace('</body>', `${script}</body>`);

    const responseWithUnregisterScript = new Response(responseBody, {
        headers: response.headers,
        status: response.status,
        statusText: response.statusText,
    });

    return responseWithUnregisterScript;
}
