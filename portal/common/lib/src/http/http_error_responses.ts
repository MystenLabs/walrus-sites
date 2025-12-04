// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import template_404 from "@templates/404-page.template.html" with { type: "text" }
import hash_mismatch from "@templates/hash-mismatch.html" with { type: "text" }
import { HttpStatusCodes } from "@lib/http/http_status_codes"
import template_404_fallback_if_missing from "@templates/404-page-callback-if-missing.template.html" with { type: "text" };
import { instrumentationFacade } from "@lib/instrumentation";

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
        "Page not found. We can't seem to find the page you're looking for.",
        template_404_fallback_if_missing as unknown as string,
    );
}

/**
 * Returns 503 Service Unavailable when the Sui full node RPC is unreachable.
 */
export function fullNodeFail(): Response {
    instrumentationFacade.bumpFullNodeFailRequests();
    return Response503(
        "Service temporarily unavailable",
        "Failed to contact the full node. Please try again later."
    );
}

/**
 * Returns 503 Service Unavailable when the Walrus aggregator is unreachable or fails.
 */
export function aggregatorFail(): Response {
    instrumentationFacade.bumpAggregatorFailRequests();
    return Response503(
        "Service temporarily unavailable",
        "Failed to contact the aggregator. Please try again later."
    );
}

export function resourceNotFound(): Response {
    return Response404(
        mainNotFoundErrorMessage,
        "Resource not found: The requested resource does not exist."
    );
}

/**
 * Returns 500 Internal Server Error for unhandled exceptions.
 * This catches unexpected errors that occur during request processing.
 */
// TODO: This is returned when wrong site-id (via base36). It shouldn't.
export function genericError(): Response {
    instrumentationFacade.bumpGenericErrors();
    return Response500(
        "Something went wrong",
        "An unexpected error occurred while processing your request. Please try again later."
    )
}

function Response404(message: string, secondaryMessage?: string, template: string = template_404 as unknown as string): Response {
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

/**
 * Returns a 500 Internal Server Error response.
 * Used when the portal encounters an unhandled exception or unexpected error.
 */
function Response500(message: string, secondaryMessage?: string): Response {
    const template = template_404 as unknown as string;
    const interpolated = template
        .replace("${message}", message)
        .replace("${secondaryMessage}", secondaryMessage ?? '')
    return new Response(interpolated, {
        status: HttpStatusCodes.INTERNAL_SERVER_ERROR,
        headers: {
            "Content-Type": "text/html",
        },
    });
}

/**
 * Returns a 503 Service Unavailable response.
 * Used when services (Sui full node RPC, Walrus aggregator) are unavailable or failing.
 */
function Response503(message: string, secondaryMessage?: string): Response {
    const template = template_404 as unknown as string;
    const interpolated = template
        .replace("${message}", message)
        .replace("${secondaryMessage}", secondaryMessage ?? '')
    return new Response(interpolated, {
        status: HttpStatusCodes.SERVICE_UNAVAILABLE,
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
    return new Response(hash_mismatch as unknown as string, {
        status: HttpStatusCodes.UNPROCESSABLE_CONTENT,
        headers: {
            "Content-Type": "text/html"
        }
    });
}
