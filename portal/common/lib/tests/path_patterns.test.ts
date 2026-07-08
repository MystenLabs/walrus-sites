// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, test, expect } from "vitest";
import {
    validateGlobPattern,
    validateRegexPattern,
    matchGlob,
    regexToGlobPattern,
    compareGlobSpecificity,
    countStars,
} from "@lib/path_patterns";

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
    // Whole-segment `*` and `**` matching is covered by the pattern×path matrices
    // below; these cover what the matrices don't.
    test("within-segment `*` (prefix/suffix)", () => {
        expect(matchGlob("/blog/old-*", "/blog/old-post")).toBe(true);
        expect(matchGlob("/assets/*.js", "/assets/app.js")).toBe(true);
        expect(matchGlob("/assets/*.js", "/assets/app.css")).toBe(false);
    });

    test("`**` does not match a different prefix", () => {
        expect(matchGlob("/foo/**", "/foo")).toBe(true);
        expect(matchGlob("/foo/**", "/bar")).toBe(false);
    });
});

describe("compareGlobSpecificity", () => {
    // Scored on the rewritten glob form; a negative result means the first
    // pattern is more specific (sorts first).
    test("more literal characters win", () => {
        // The extra slash makes the deeper catch-all more literal.
        expect(compareGlobSpecificity("/something/else/**/*", "/something/else/**")).toBeLessThan(
            0,
        );
        expect(compareGlobSpecificity("/docs/**/*", "/**/*")).toBeLessThan(0);
    });

    test("with equal literals, fewer wildcards win", () => {
        // `/a/b` and `/a/b/**` both have four literal chars; the exact one has no star.
        expect(compareGlobSpecificity("/a/b", "/a/b/**")).toBeLessThan(0);
    });

    test("sorts a list most-specific first", () => {
        const sorted = ["/**/*", "/something/else/**", "/something/else/**/*"].sort(
            compareGlobSpecificity,
        );
        expect(sorted).toEqual(["/something/else/**/*", "/something/else/**", "/**/*"]);
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

// Path×pattern matrices: one row per path, one column per pattern. Each cell is
// `-` (no match) or a rank among the patterns matching that row's path — `0` is
// the winner, `1` wins if `0` weren't there, and so on (compareGlobSpecificity
// order, ties to the first column, like the router). Run with
// `vitest run … --disableConsoleIntercept` to see the printed tables.
type MatrixCell = number | "-";

function checkMatrix(title: string, patterns: string[], rows: [string, MatrixCell[]][]): void {
    const actual: MatrixCell[][] = rows.map(([path]) => {
        // Patterns matching this path, ranked by specificity: 0 wins, 1 wins if 0
        // weren't there, etc. Ties keep column order (stable sort), like the router.
        const ranked = patterns
            .map((pattern, col) => ({ pattern, col }))
            .filter(({ pattern }) => matchGlob(pattern, path))
            .sort((a, b) => compareGlobSpecificity(a.pattern, b.pattern));
        const rankByCol = new Map(ranked.map(({ col }, rank) => [col, rank]));
        return patterns.map((_, col) => rankByCol.get(col) ?? "-");
    });
    expect(actual).toEqual(rows.map(([, row]) => row));

    const patW = Math.max(...rows.map(([p]) => p.length));
    const colW = patterns.map((p) => Math.max(p.length, 1));
    const line = (label: string, cells: string[]): string =>
        label.padEnd(patW) + "  " + cells.map((c, i) => c.padEnd(colW[i])).join("  ");
    const table = [
        line("", patterns),
        ...actual.map((cells, r) => line(rows[r][0], cells.map(String))),
    ];
    console.log(`\n${title}\n${table.join("\n")}\n`);
}

// Matrix0 — root catch-all. `/*` and `*` are redirect-only (routes rewrite them
// to `/**/*` and `/**`). Bare `*` matches nothing: every path has a leading
// slash, so ≥2 segments, and `*` is a single segment.
describe("Matrix0 — root catch-all", () => {
    test("crowns the most-specific root catch-all", () => {
        checkMatrix(
            "Matrix0 — root catch-all",
            ["/", "/**", "/**/*", "**", "/*", "*"],
            [
                ["/", [0, 3, 2, 4, 1, "-"]],
                ["/foo", ["-", 2, 1, 3, 0, "-"]],
                ["/foo/", ["-", 1, 0, 2, "-", "-"]],
                ["/foo/bar", ["-", 1, 0, 2, "-", "-"]],
            ],
        );
    });
});

// Matrix1 — middle wildcards. Exact / finite-segment patterns beat the globstar
// catch-all they overlap.
describe("Matrix1 — middle wildcards", () => {
    test("crowns the most-specific middle wildcard", () => {
        checkMatrix(
            "Matrix1 — middle wildcards",
            ["/foo/bar", "/foo/*/bar", "/foo/**/bar", "/foo/**/*/bar"],
            [
                ["/foo/bar", [0, "-", 1, "-"]],
                ["/foo/x/bar", ["-", 0, 2, 1]],
                ["/foo/x/y/bar", ["-", "-", 1, 0]],
                ["/foo/x/y/z/bar", ["-", "-", 1, 0]],
            ],
        );
    });
});

// Matrix2 — trailing wildcards. Every tail (`/foo`, `/foo/*`, `/foo/*/*`,
// `/foo/**`, `/foo/**/*`, `/foo/**/*/*`) paired with its trailing-slash variant,
// against growing path depth. The finite tail always beats its globstar twin.
describe("Matrix2 — trailing wildcards", () => {
    test("crowns the most-specific trailing wildcard", () => {
        checkMatrix(
            "Matrix2 — trailing wildcards",
            [
                "/foo",
                "/foo/",
                "/foo/*",
                "/foo/*/",
                "/foo/*/*",
                "/foo/*/*/",
                "/foo/**",
                "/foo/**/",
                "/foo/**/*",
                "/foo/**/*/",
                "/foo/**/*/*",
                "/foo/**/*/*/",
            ],
            [
                ["/foo", [0, "-", "-", "-", "-", "-", 1, "-", "-", "-", "-", "-"]],
                ["/foo/", ["-", 0, 1, "-", "-", "-", 4, 2, 3, "-", "-", "-"]],
                ["/foo/bar", ["-", "-", 0, "-", "-", "-", 2, "-", 1, "-", "-", "-"]],
                ["/foo/bar/", ["-", "-", "-", 0, 1, "-", 6, 4, 5, 2, 3, "-"]],
                ["/foo/bar/baz", ["-", "-", "-", "-", 0, "-", 3, "-", 2, "-", 1, "-"]],
                ["/foo/bar/baz/", ["-", "-", "-", "-", "-", 0, 6, 4, 5, 2, 3, 1]],
            ],
        );
    });
});
