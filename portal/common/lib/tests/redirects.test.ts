// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Import necessary functions and types
import { describe, expect, test } from "vitest";
import { redirectToPortalURLResponse, redirectToAggregatorUrlResponse } from "@lib/redirects";
import { DomainDetails } from "@lib/types/index";

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
