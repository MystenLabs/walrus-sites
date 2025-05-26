// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import template_404 from "../../html_templates/404-page.template.html" with { type: "text" }
import hash_mismatch from "../../html_templates/hash-mismatch.html" with { type: "text" }
import { HttpStatusCodes } from "./http_status_codes"
import template_404_fallback_if_missing from "../../html_templates/404-page-callback-if-missing.template.html" with { type: "text" };

const mainNotFoundErrorMessage = "Well, this is awkward." //You have reached the end of the internet, please turn back!"

export function siteNotFound(): Response {
    return Response404(
        mainNotFoundErrorMessage,
        "We promise our storage protocol is rock-solid, but this page seems to have gone on a coffee break." //Invalid URL: The object ID is not a valid Walrus Site."
    );
}

export function noObjectIdFound(): Response {
    return Response404(
        mainNotFoundErrorMessage,
        "Invalid URL: Walrus Site not found!"
    );
}

export function custom404NotFound(): Response {
    return Response404(
        "Oops!",
        "Page not found. We can’t seem to find the page you’re looking for.",
        template_404_fallback_if_missing,
    );
}

export function fullNodeFail(): Response {
    return Response404("Failed to contact the full node.");
}

export function resourceNotFound(): Response {
    return Response404(
        mainNotFoundErrorMessage,
        "Resource not found: The requested resource does not exist."
    );
}

export function genericError(): Response {
    return Response404(
        mainNotFoundErrorMessage
    )
}

function Response404(message: string, secondaryMessage?: string, template: string = template_404): Response {
    const interpolated = template
        .replace("${message}", message)
        .replace("${secondaryMessage}", secondaryMessage ?? '')
    return new Response(interpolated, {
        status: 404,
        headers: {
            "Content-Type": "text/html",
        },
    });
}

export function bringYourOwnDomainDoesNotSupportSubdomainsYet(attemptedSite: String): Response {
	return Response404(
		`This portal does not serve any other Walrus Sites!`,
		`Please try browsing https://${attemptedSite}.wal.app`
	)
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
