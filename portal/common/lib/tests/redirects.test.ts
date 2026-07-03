// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Import necessary functions and types
import { describe, expect, test } from "vitest";
import {
    redirectToPortalURLResponse,
    redirectToAggregatorUrlResponse,
    checkRedirect,
} from "@lib/redirects";
import { DomainDetails } from "@lib/types/index";
import { SuiClientTypes } from "@mysten/sui/client";

const mockAggregatorUrl = "https://aggregator.walrus-testnet.walrus.space";
const redirectToPortalURLTestCases: [string, DomainDetails, string][] = [
    [
        "https://example.com",
        { subdomain: "subname", path: "/index.html" },
        "https://subname.example.com/index.html",
    ],
    [
        "https://wal.app",
        { subdomain: "name", path: "/index.html" },
        "https://name.wal.app/index.html",
    ],
    [
        "http://localhost:8080",
        { subdomain: "docs", path: "/css/print.css" },
        "http://docs.localhost:8080/css/print.css",
    ],
    [
        "https://portalname.co.uk",
        { subdomain: "subsubname.subname", path: "/index.html" },
        "https://subsubname.subname.portalname.co.uk/index.html",
    ],
];

describe("redirectToPortalURLResponse", () => {
    redirectToPortalURLTestCases.forEach(([input, path, expected]) => {
        test(`${input} with subdomain: ${path.subdomain} and path: ${path.path} -> ${expected}`, () => {
            const scope = new URL(input) as URL;
            const response = redirectToPortalURLResponse(scope, path);
            expect(response.status).toBe(302);
            expect(response.headers.get("Location")).toBe(expected);
        });
    });
});

const redirectToAggregatorUrlTestCases: [string, string, string][] = [
    [
        "https://example.com",
        "12345",
        "https://aggregator.walrus-testnet.walrus.space/v1/blobs/12345",
    ],
    [
        "https://wal.app",
        "blob-id",
        "https://aggregator.walrus-testnet.walrus.space/v1/blobs/blob-id",
    ],
    [
        "http://localhost:8080",
        "abcde",
        "https://aggregator.walrus-testnet.walrus.space/v1/blobs/abcde",
    ],
];

describe("redirectToAggregatorUrlResponse", () => {
    redirectToAggregatorUrlTestCases.forEach(([input, blobId, expected]) => {
        test(`${input} with blobId: ${blobId} -> ${expected}`, () => {
            const response = redirectToAggregatorUrlResponse(blobId, mockAggregatorUrl);
            expect(response.status).toBe(302);
            expect(response.headers.get("Location")).toBe(expected);
        });
    });
});

// Builds the minimal object `checkRedirect` reads. `display` is passed verbatim
// so tests can mirror the real gRPC `core.getObjects` shapes captured on-chain.
// Partial mock — cast because SuiClientTypes.Object also requires
// owner/type/previousTransaction/objectBcs/json, unused by this check.
function objectWithDisplay(display: unknown): SuiClientTypes.Object<{ display: true }> {
    return {
        objectId: "0x1",
        version: "1",
        digest: "d",
        display,
    } as unknown as SuiClientTypes.Object<{
        display: true;
    }>;
}

const REDIRECT_TARGET = "0x205f06c6cf7b573ed96a7988ac539bba09d3a384b84dec71a7662b66180d8272";

describe("checkRedirect", () => {
    test("returns the target when the redirect field is set (migrated v2 object)", () => {
        // Real shape from the migrated Redirector object.
        const object = objectWithDisplay({
            output: { "walrus site address": REDIRECT_TARGET },
            errors: null,
        });
        expect(checkRedirect(object)).toBe(REDIRECT_TARGET);
    });

    test("returns null for a normal Site (no redirect field)", () => {
        // Real shape from docs.wal.app: all Site fields present, no redirect key.
        const object = objectWithDisplay({
            output: {
                name: "Walrus Documentation",
                link: null,
                image_url: "https://www.walrus.xyz/walrus-site",
                description: "A walrus site created using Walrus and Sui!",
                project_url: null,
                creator: null,
            },
            errors: null,
        });
        expect(checkRedirect(object)).toBeNull();
    });

    test("returns null for a legacy v1 Display (gRPC renders it as undefined)", () => {
        // Real shape captured before the v2 migration: gRPC omits the display.
        expect(checkRedirect(objectWithDisplay(undefined))).toBeNull();
    });

    test("returns null when the type has no Display template (display null)", () => {
        expect(checkRedirect(objectWithDisplay(null))).toBeNull();
    });

    test("returns null when the field renders empty (output null)", () => {
        expect(checkRedirect(objectWithDisplay({ output: null, errors: null }))).toBeNull();
    });

    test("returns null when the field renders a non-string (Display v2 JSON value)", () => {
        const object = objectWithDisplay({
            output: { "walrus site address": { nested: "json" } },
            errors: null,
        });
        expect(checkRedirect(object)).toBeNull();
    });
});
