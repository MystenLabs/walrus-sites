// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from "vitest";
import { getDomain, getSubdomainAndPath } from "@lib/domain_parsing";
import { DomainDetails } from "@lib/types";

const PORTAL_DOMAIN_NAME_LENGTH = 21;

const getDomainTestCases: [string, string][] = [
    ["https://example.com", "example.com"],
    ["https://suinsname.localhost:8080", "localhost"],
    ["https://subname.suinsname.localhost:8080", "localhost"],
    ["https://flatland.wal.app/", "wal.app"],
    ["https://4snh0c0o7quicfzokqpsmuchtgitnukme1q680o1s1nfn325sr.wal.app/", "wal.app"],
    [
        "https://4snh0c0o7quicfzokqpsmuchtgitnukme1q680o1s1nfn325sr.portalname.co.uk/",
        "portalname.co.uk",
    ],
    ["https://subname.suinsname.portalname.co.uk/", "portalname.co.uk"],
    ["https://subsubname.subname.suinsname.portalname.co.uk/", "portalname.co.uk"],
];

describe("getDomain", () => {
    test.each(getDomainTestCases)("%s -> %s", (input, expected) => {
        const domain = getDomain(new URL(input) as URL);
        expect(domain).toEqual(expected);
    });
});

const getDomainWithPortalNameLengthTestCases: [string, string][] = [
    ["https://sw-tnet.blocksite.net", "sw-tnet.blocksite.net"],
    ["https://subname.sw-tnet.blocksite.net", "sw-tnet.blocksite.net"],
];

describe("getDomain with portal name length", () => {
    test.each(getDomainWithPortalNameLengthTestCases)("%s -> %s", (input, expected) => {
        const domain = getDomain(new URL(input) as URL, PORTAL_DOMAIN_NAME_LENGTH);
        expect(domain).toEqual(expected);
    });
});

const getSubdomainAndPathTestCases: [string, DomainDetails][] = [
	["https://subname.name.wal.app/", { subdomain: "subname.name", path: "/index.html" }],
	["https://name.wal.app/", { subdomain: "name", path: "/index.html" }],
	["http://name.localhost:8080/", { subdomain: "name", path: "/index.html" }],
	["http://flatland.localhost:8080/", { subdomain: "flatland", path: "/index.html" }],
	[
		"http://subname.suinsname.localhost:8080/",
		{ subdomain: "subname.suinsname", path: "/index.html" },
	],
	[
		"https://subsubname.subname.suinsname.portalname.co.uk/",
		{ subdomain: "subsubname.subname.suinsname", path: "/index.html" },
	],
	["http://docs.localhost/css/print.css", { subdomain: "docs", path: "/css/print.css" }],
	[
		"http://docs.localhost/assets/index-a242f32b.js",
		{ subdomain: "docs", path: "/assets/index-a242f32b.js" },
	],
	[
		"https://mystenlabs-logos.wal.app/02_Horizontal%20Logo/index.html",
		{ subdomain: "mystenlabs-logos", path: "/02_Horizontal Logo/index.html" },
	],
	[
		"http://my-site.localhost/files/report%2D2024.pdf",
		{ subdomain: "my-site", path: "/files/report-2024.pdf" },
	],
	[
		"http://my-site.localhost/files/report-2024.pdf",
		{ subdomain: "my-site", path: "/files/report-2024.pdf" },
	],
];

describe("getSubdomainAndPath", () => {
	test.each(getSubdomainAndPathTestCases)("%s -> %s", (input, path) => {
		expect(getSubdomainAndPath(new URL(input) as URL)).toEqual(path);
	});
});

const getSubdomainAndPathWithPortalLengthTestCases: [string, DomainDetails][] = [
    [
        "https://subname.name.sw-tnet.blocksite.net/",
        { subdomain: "subname.name", path: "/index.html" },
    ],
    ["https://name.sw-tnet.blocksite.net/", { subdomain: "name", path: "/index.html" }],
];
describe("getSubdomainAndPath", () => {
    test.each(getSubdomainAndPathWithPortalLengthTestCases)(
        "%s -> %s",
        (input, path) => {
            expect(getSubdomainAndPath(new URL(input) as URL, PORTAL_DOMAIN_NAME_LENGTH)).toEqual(
                path,
            );
        },
    );
});
