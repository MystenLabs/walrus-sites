// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from "vitest";
import { bcs } from "@mysten/bcs";
import {
    DynamicFieldStruct,
    RedirectStruct,
    RedirectsStruct,
    RoutesStruct,
} from "@lib/bcs_data_parsing";

describe("RedirectsStruct BCS round-trip", () => {
    test("RedirectStruct round-trips correctly", () => {
        const redirect = { location: "/new-page", status_code: 301 };
        const bytes = RedirectStruct.serialize(redirect).toBytes();
        const parsed = RedirectStruct.parse(bytes);
        expect(parsed).toEqual(redirect);
    });

    test("RedirectsStruct round-trips correctly", () => {
        const redirects = {
            redirect_list: new Map([
                ["/old", { location: "/new", status_code: 301 }],
                ["/temp", { location: "https://example.com", status_code: 302 }],
                ["/moved", { location: "/destination", status_code: 308 }],
            ]),
        };
        const bytes = RedirectsStruct.serialize(redirects).toBytes();
        const parsed = RedirectsStruct.parse(bytes);
        expect(parsed.redirect_list.size).toBe(3);
        expect(parsed.redirect_list.get("/old")).toEqual({ location: "/new", status_code: 301 });
        expect(parsed.redirect_list.get("/temp")).toEqual({
            location: "https://example.com",
            status_code: 302,
        });
        expect(parsed.redirect_list.get("/moved")).toEqual({
            location: "/destination",
            status_code: 308,
        });
    });

    test("RedirectsStruct round-trips through DynamicFieldStruct", () => {
        const redirects = {
            redirect_list: new Map([
                ["/old", { location: "/new", status_code: 301 }],
                ["/temp", { location: "https://example.com", status_code: 302 }],
            ]),
        };
        const df = {
            parentId: "0x" + "00".repeat(32),
            name: [...Buffer.from("redirects")],
            value: redirects,
        };
        const bcsType = DynamicFieldStruct(bcs.vector(bcs.u8()), RedirectsStruct);
        const bytes = bcsType.serialize(df).toBytes();
        const parsed = bcsType.parse(bytes);
        expect(parsed.value.redirect_list.size).toBe(2);
        expect(parsed.value.redirect_list.get("/old")).toEqual({
            location: "/new",
            status_code: 301,
        });
        expect(parsed.value.redirect_list.get("/temp")).toEqual({
            location: "https://example.com",
            status_code: 302,
        });
    });

    test("empty RedirectsStruct round-trips correctly", () => {
        const redirects = { redirect_list: new Map() };
        const bytes = RedirectsStruct.serialize(redirects).toBytes();
        const parsed = RedirectsStruct.parse(bytes);
        expect(parsed.redirect_list.size).toBe(0);
    });
});

describe("RoutesStruct BCS round-trip", () => {
    test("RoutesStruct round-trips through DynamicFieldStruct", () => {
        const routes = {
            routes_list: new Map([
                ["/*", "/index.html"],
                ["/blog/*", "/blog/index.html"],
            ]),
        };
        const df = {
            parentId: "0x" + "00".repeat(32),
            name: [...Buffer.from("routes")],
            value: routes,
        };
        const bcsType = DynamicFieldStruct(bcs.vector(bcs.u8()), RoutesStruct);
        const bytes = bcsType.serialize(df).toBytes();
        const parsed = bcsType.parse(bytes);
        expect(parsed.value.routes_list.size).toBe(2);
        expect(parsed.value.routes_list.get("/*")).toBe("/index.html");
    });
});
