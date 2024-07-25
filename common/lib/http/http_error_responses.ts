// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import template_404 from "@static/404-page.template.html";

export function siteNotFound(): Response {
    return Response404(
        "This page does not exist - the object ID is not a valid Walrus Site."
    );
}

export function noObjectIdFound(): Response {
    return Response404("This page does not exist - no object ID could be found.");
}

export function fullNodeFail(): Response {
    return Response404("Failed to contact the full node.");
}

function Response404(message: String): Response {
    console.log();
    return new Response(template_404.replace("${message}", message), {
        status: 404,
        headers: {
            "Content-Type": "text/html",
        },
    });
}
