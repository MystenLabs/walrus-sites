// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Tests for HTTP error response functions.
 *
 * These tests verify that the portal returns correct HTTP status codes
 * for different error scenarios.
 *
 * - Client errors (4xx): User/request issues (invalid URLs, missing resources)
 * - Server errors (5xx): Infrastructure/backend failures (RPC down, aggregator unreachable)
 *
 * When modifying error response functions in http_error_responses.ts, run these tests
 * to ensure status codes remain correct. Returning wrong status codes (e.g., 404 instead of 503)
 * will mask infrastructure failures and break monitoring/alerting.
 *
 * Run tests:
 *   bun test http_error_responses.test.ts
 *
 * See: portal/common/lib/src/http/http_error_responses.ts
 *      portal/common/lib/src/http/http_status_codes.ts
 */

import { describe, it, expect } from "vitest";
import {
    siteNotFound,
    noObjectIdFound,
    custom404NotFound,
    fullNodeFail,
    aggregatorFail,
    resourceNotFound,
    genericError,
    generateHashErrorResponse,
    bringYourOwnDomainDoesNotSupportSubdomainsYet,
} from "@lib/http/http_error_responses";
import { HttpStatusCodes } from "@lib/http/http_status_codes";

describe("HTTP Error Responses - Status Codes", () => {
    describe("404 Not Found responses", () => {
        it("siteNotFound() should return 404", async () => {
            const response = siteNotFound();
            expect(response.status).toBe(404);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });

        it("noObjectIdFound() should return 404", async () => {
            const response = noObjectIdFound();
            expect(response.status).toBe(404);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });

        it("custom404NotFound() should return 404", async () => {
            const response = custom404NotFound();
            expect(response.status).toBe(404);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });

        it("resourceNotFound() should return 404", async () => {
            const response = resourceNotFound();
            expect(response.status).toBe(404);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });

        it("bringYourOwnDomainDoesNotSupportSubdomainsYet() should return 404", async () => {
            const response = bringYourOwnDomainDoesNotSupportSubdomainsYet("test-site");
            expect(response.status).toBe(404);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });
    });

    describe("500 Internal Server Error responses", () => {
        it("genericError() should return 500, not 404", async () => {
            const response = genericError();
            expect(response.status).toBe(HttpStatusCodes.INTERNAL_SERVER_ERROR);
            expect(response.status).toBe(500);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });

        it("genericError() response body should contain error message", async () => {
            const response = genericError();
            const text = await response.text();
            expect(text).toContain("An unexpected error occurred");
        });
    });

    describe("503 Service Unavailable responses", () => {
        it("fullNodeFail() should return 503, not 404", async () => {
            const response = fullNodeFail();
            expect(response.status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
            expect(response.status).toBe(503);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });

        it("fullNodeFail() response body should indicate service unavailability", async () => {
            const response = fullNodeFail();
            const text = await response.text();
            expect(text).toContain("Failed to contact the full node");
        });

        it("aggregatorFail() should return 503, not 404", async () => {
            const response = aggregatorFail();
            expect(response.status).toBe(HttpStatusCodes.SERVICE_UNAVAILABLE);
            expect(response.status).toBe(503);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });

        it("aggregatorFail() response body should indicate storage network unavailability", async () => {
            const response = aggregatorFail();
            const text = await response.text();
            expect(text).toContain("Failed to contact the aggregator");
        });
    });

    describe("422 Unprocessable Content responses", () => {
        it("generateHashErrorResponse() should return 422", async () => {
            const response = generateHashErrorResponse();
            expect(response.status).toBe(HttpStatusCodes.UNPROCESSABLE_CONTENT);
            expect(response.status).toBe(422);
            expect(response.headers.get("Content-Type")).toBe("text/html");
        });
    });
});

describe("HTTP Error Responses - Content", () => {
    it("all error responses should return valid Response objects", () => {
        const responses = [
            siteNotFound(),
            noObjectIdFound(),
            custom404NotFound(),
            fullNodeFail(),
            aggregatorFail(),
            resourceNotFound(),
            genericError(),
            generateHashErrorResponse(),
            bringYourOwnDomainDoesNotSupportSubdomainsYet("example"),
        ];

        responses.forEach((response) => {
            expect(response).toBeInstanceOf(Response);
            expect(response.headers.get("Content-Type")).toBeTruthy();
        });
    });

    it("all error responses should have non-empty body", async () => {
        const responses = [
            siteNotFound(),
            noObjectIdFound(),
            custom404NotFound(),
            fullNodeFail(),
            aggregatorFail(),
            resourceNotFound(),
            genericError(),
            generateHashErrorResponse(),
        ];

        for (const response of responses) {
            const text = await response.text();
            expect(text.length).toBeGreaterThan(0);
        }
    });

    it("bringYourOwnDomainDoesNotSupportSubdomainsYet() should include the attempted site name", async () => {
        const siteName = "my-test-site";
        const response = bringYourOwnDomainDoesNotSupportSubdomainsYet(siteName);
        const text = await response.text();
        expect(text).toContain(siteName);
    });
});

describe("HTTP Error Responses - Status Code Classification", () => {
    it("should correctly separate client errors (4xx) from server errors (5xx)", () => {
        // Client errors (4xx) - these are user/request issues
        const clientErrors = [
            siteNotFound(),
            noObjectIdFound(),
            custom404NotFound(),
            resourceNotFound(),
            bringYourOwnDomainDoesNotSupportSubdomainsYet("test"),
            generateHashErrorResponse(),
        ];

        clientErrors.forEach((response) => {
            expect(response.status).toBeGreaterThanOrEqual(400);
            expect(response.status).toBeLessThan(500);
        });

        // Server errors (5xx) - these are infrastructure/backend issues
        const serverErrors = [
            fullNodeFail(),      // Sui RPC unreachable
            aggregatorFail(),    // aggregator unreachable
            genericError(),      // Unhandled exception
        ];

        serverErrors.forEach((response) => {
            expect(response.status).toBeGreaterThanOrEqual(500);
            expect(response.status).toBeLessThan(600);
        });
    });

    it("infrastructure failures should never return 404", () => {
        // These represent infrastructure/backend failures, not "not found" errors
        const infrastructureFailures = [
            fullNodeFail(),      // Sui RPC is down/unreachable
            aggregatorFail(),    // aggregator is down/unreachable
        ];

        infrastructureFailures.forEach((response) => {
            expect(response.status).not.toBe(404);
            expect(response.status).toBe(503);
        });
    });

    it("unhandled exceptions should never return 404", () => {
        const response = genericError();
        expect(response.status).not.toBe(404);
        expect(response.status).toBe(500);
    });
});
