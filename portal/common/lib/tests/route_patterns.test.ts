// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, test, expect } from "vitest";
import {
    validateGlobPattern,
    validateRegexPattern,
    matchGlob,
    regexToGlobPattern,
    countStars,
} from "@lib/route_patterns";

describe("countStars", () => {
    test.each([
        ["", 0],
        ["/foo", 0],
        ["/foo/*", 1],
        ["/**", 2],
        ["a*b*", 2],
    ])("countStars(%j) = %i", (input, expected) => {
        expect(countStars(input as string)).toBe(expected as number);
    });
});

describe("validatePattern — glob mode", () => {
    // A segment may use a single `*`, or be a whole-segment `**`. Non-wildcard
    // characters (incl. `( ) [ ] ! +`) are literals, so they never affect this.
    test.each([
        "/*",
        "/**",
        "/foo",
        "/foo/*",
        "/assets/*.js",
        "/blog/old-*",
        "/a/*/b/*",
        "/foo/**/bar",
        "/foo/**/*",
        "/legacy/**/*",
        "/wiki/Foo_(bar)",
        "/list[0]",
        "/path!/+",
    ])("accepts %j", (pattern) => expect(validateGlobPattern(pattern).ok).toBe(true));

    // `**` is only valid as a whole segment, so a `**` glued to a literal — or
    // any segment with more than one `*` — is a malformed token and rejected.
    test.each(["/foo/bar**", "a*b*", "/x/a*a*a*/y", "***", "/**bar**/x", "/*foo*"])(
        "rejects %j (more than one '*' in a segment)",
        (pattern) => expect(validateGlobPattern(pattern).ok).toBe(false),
    );

    // One globstar is linear; multiple whole-segment globstars backtrack
    // combinatorially even though each segment is well-formed.
    test.each(["/foo/**/*/**/bar", "/a/**/b/**", "/**/x/**/y"])(
        "rejects %j (2+ globstars)",
        (pattern) => expect(validateGlobPattern(pattern).ok).toBe(false),
    );
});

describe("validatePattern — regex mode", () => {
    // The legacy regex crosses `/`, so the cap is on total stars (≤2). `.` is
    // any-char (today's behaviour) and appears in real routes (`.html`).
    test.each(["/*", "/foo", "/foo/*", "/a/*/b/*", "a*b*", "/index.html"])(
        "accepts %j",
        (pattern) => expect(validateRegexPattern(pattern).ok).toBe(true),
    );
    test.each(["/a/*/b/*/c/*", "/a*b*c*", "***"])("rejects %j (3+ stars total)", (pattern) =>
        expect(validateRegexPattern(pattern).ok).toBe(false),
    );
    // Every regex metacharacter is rejected: they crash `RegExp` (`( ) [ ] { }`),
    // backtrack (`+ ?`), or don't work as literals here (`^ $ | \`).
    test.each(["/foo(", "/foo)", "/foo[a]", "/foo{2}", "/a+b", "/a?b", "/a|b", "/p$", "/a\\b"])(
        "rejects %j (regex metacharacter)",
        (pattern) => expect(validateRegexPattern(pattern).ok).toBe(false),
    );
});

describe("matchGlob — literals", () => {
    // Every non-wildcard character matches literally, including delimiters that
    // a glob library would treat as syntax (balanced or not).
    test.each([
        ["/wiki/Foo_(bar)", "/wiki/Foo_(bar)", "/wiki/Foo_bar"],
        ["/list[0]", "/list[0]", "/list0"],
        ["/item!", "/item!", "/itemX"],
        ["/a+b", "/a+b", "/aaab"],
        ["/a.b", "/a.b", "/aXb"],
        ["/foo(/bar", "/foo(/bar", "/fooX/bar"],
    ])("%j matches literally", (pattern, hit, miss) => {
        expect(matchGlob(pattern, hit)).toBe(true);
        expect(matchGlob(pattern, miss)).toBe(false);
    });
});

describe("matchGlob — wildcards", () => {
    test("`*` matches within one segment only", () => {
        expect(matchGlob("/p/*", "/p/x")).toBe(true);
        expect(matchGlob("/p/*", "/p/x/y")).toBe(false);
    });

    test("within-segment `*` (prefix/suffix)", () => {
        expect(matchGlob("/blog/old-*", "/blog/old-post")).toBe(true);
        expect(matchGlob("/assets/*.js", "/assets/app.js")).toBe(true);
        expect(matchGlob("/assets/*.js", "/assets/app.css")).toBe(false);
    });

    test("mid-pattern `*` stays single-segment", () => {
        expect(matchGlob("/forms/*/admin", "/forms/123/admin")).toBe(true);
        expect(matchGlob("/forms/*/admin", "/forms/a/b/admin")).toBe(false);
    });

    test("`**` matches any run of segments, including zero", () => {
        expect(matchGlob("/foo/**", "/foo")).toBe(true);
        expect(matchGlob("/foo/**", "/foo/x")).toBe(true);
        expect(matchGlob("/foo/**", "/foo/x/y/z")).toBe(true);
        expect(matchGlob("/foo/**", "/bar")).toBe(false);
    });

    test("`/foo/**/*` requires one more segment than `/foo/**`", () => {
        // The "extra slash" the most-exact tiebreaker relies on.
        expect(matchGlob("/foo/**/*", "/foo")).toBe(false);
        expect(matchGlob("/foo/**/*", "/foo/x")).toBe(true);
        expect(matchGlob("/foo/**/*", "/foo/x/y")).toBe(true);
    });

    test("with no `**`, segments must line up one-to-one", () => {
        expect(matchGlob("/a/b", "/a/b")).toBe(true);
        expect(matchGlob("/a/b", "/a/b/c")).toBe(false);
        expect(matchGlob("/a/b", "/a")).toBe(false);
    });
});

describe("regexToGlobPattern", () => {
    test.each([
        ["*", "/**"], // bare catch-all matches everything
        ["/*", "/**/*"], // root catch-all requires at least one segment
        ["/docs/*", "/docs/**/*"], // trailing star requires the slash + a segment
        ["/forms/*/admin", "/forms/*/admin"], // mid-pattern star stays single-segment
        ["/blog/old-*", "/blog/old-*"], // within-segment star is untouched
        ["/foo/**", "/foo/**"], // already a glob, untouched
        ["/about", "/about"], // no wildcard, untouched
    ])("regexToGlobPattern(%j) = %j", (input, expected) => {
        expect(regexToGlobPattern(input)).toBe(expected);
    });
});
