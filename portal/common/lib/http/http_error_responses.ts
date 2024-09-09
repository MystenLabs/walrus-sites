// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import template_404 from "../../static/404-page.template.html";

export function siteNotFound(): Response {
    return Response404(
        "You have reached the end of the internet, please turn back!",
        "Invalid URL: The object ID is not a valid Walrus Site."
    );
}

export function noObjectIdFound(): Response {
    return Response404(
        "You have reached the end of the internet, please turn back!",
        "Invalid URL: No object ID could be found."
    );
}

export function fullNodeFail(): Response {
    return Response404("Failed to contact the full node.");
}

function Response404(message: String, secondaryMessage?: String): Response {
    const interpolated = template_404
        .replace("${message}", message)
        .replace("${secondaryMessage}", secondaryMessage ?? '')
    return new Response(interpolated, {
        status: 404,
        headers: {
            "Content-Type": "text/html",
        },
    });
}
