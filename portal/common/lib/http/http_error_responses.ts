// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import template_404 from "../../static/404-page.template.html";
import hash_mismatch from "../../static/hash-mismatch.html"
import { HttpStatusCodes } from "./http_status_codes";

const mainNotFoundErrorMessage = "You have reached the end of the internet, please turn back!"

export function siteNotFound(): Response {
    return Response404(
        mainNotFoundErrorMessage,
        "Invalid URL: The object ID is not a valid Walrus Site."
    );
}

export function noObjectIdFound(): Response {
    return Response404(
        mainNotFoundErrorMessage,
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

/**
* Returns the html page that displays an alert to the user
* regarding a mismatch between the aggregator response and
* the blob hash (checksum).
*/
export function generateHashErrorResponse(): Response {
    return new Response(hash_mismatch, {
        status: HttpStatusCodes.UNPROCESSABLE_CONTENT,
        headers: {
            "Content-Type": "text/html"
        }
    });
}
