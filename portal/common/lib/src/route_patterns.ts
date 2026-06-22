// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Validating and matching owner-supplied route/redirect patterns.
 *
 * Grammar: `*` matches within one path segment, `**` (a whole segment) matches
 * across segments, and every other character is a literal. `*` is reserved, so a
 * literal `*` in a path can't be targeted; every other URL path character
 * (`( ) [ ] ! +` ...) just works.
 *
 * Wildcards are capped so a crafted pattern can't freeze the event loop (ReDoS):
 *  - glob (redirects always; routes when the flag is on): each segment has at
 *    most one `*` or is a whole-segment `**`, and at most one `**` per pattern.
 *    Those two guarantees let `matchGlob` anchor without any backtracking.
 *  - legacy regex (routes, flag off): the pattern is passed to `RegExp` with `*`
 *    turned into `.*` (crossing `/`), so it accepts only plain path characters
 *    plus `*` (the wildcard) and `.` (today's any-char). Every other regex
 *    metacharacter is rejected — they crash `RegExp` (`( ) [ ] { }`), backtrack
 *    (`+ ?`), or just don't work as literals here (`^ $ | \` — none are valid in
 *    a URL path anyway). Such patterns match literally instead under the glob
 *    matcher, which treats every non-`*` character as a literal.
 */

/** Max total `*` characters in a legacy-regex pattern (glob is bounded per segment). */
const MAX_STARS = 2;

/** Max whole-segment `**` globstars in a glob pattern (each adds backtracking). */
const MAX_GLOBSTARS = 1;

/**
 * Regex metacharacters rejected in legacy-regex route patterns. Only `*` (the
 * wildcard, translated to `.*`) and `.` (today's any-char) are allowed through.
 */
const ILLEGAL_REGEX_CHARS = /[()[\]{}+?^$|\\]/;

export interface PatternValidation {
    ok: boolean;
    /** Why the pattern was rejected (for logging); omitted when ok. */
    reason?: string;
}

/** Number of `*` characters in `text`. */
export function countStars(text: string): number {
    let count = 0;
    for (const char of text) {
        if (char === "*") count += 1;
    }
    return count;
}

/**
 * Validates a legacy-regex route pattern (the matcher used while the glob flag is
 * off). Rejects regex metacharacters and caps the total wildcard count. Callers
 * skip (and log) patterns that fail rather than feeding them to the matcher.
 */
export function validateRegexPattern(pattern: string): PatternValidation {
    const illegal = pattern.match(ILLEGAL_REGEX_CHARS);
    if (illegal) {
        return { ok: false, reason: `unsupported character "${illegal[0]}"` };
    }
    const total = countStars(pattern);
    if (total > MAX_STARS) {
        return { ok: false, reason: `${total} '*' characters (max ${MAX_STARS} in total)` };
    }
    return { ok: true };
}

/**
 * Validates a glob route/redirect pattern. Each segment may use at most one `*`
 * or be a whole-segment `**`, and a pattern may have at most one `**`. Callers
 * skip (and log) patterns that fail rather than feeding them to the matcher.
 */
export function validateGlobPattern(pattern: string): PatternValidation {
    let globstars = 0;
    for (const segment of pattern.split("/")) {
        if (segment === "**") {
            globstars += 1;
            continue;
        }
        // Any other segment may use at most one `*` (so `bar**`, `a*b*`, `***`
        // are rejected: `**` is only valid as a whole segment).
        if (countStars(segment) > 1) {
            return {
                ok: false,
                reason: `segment "${segment}" may use at most one '*', or be a whole-segment '**'`,
            };
        }
    }
    if (globstars > MAX_GLOBSTARS) {
        return { ok: false, reason: `${globstars} '**' globstars (max ${MAX_GLOBSTARS})` };
    }
    return { ok: true };
}

/**
 * Matches one path segment against a pattern segment. Validation guarantees at
 * most one `*`, so the segment is `prefix*suffix`: the text must start with the
 * prefix, end with the suffix, and be long enough for both not to overlap.
 */
function matchSegment(patternSegment: string, textSegment: string): boolean {
    const star = patternSegment.indexOf("*");
    if (star === -1) return patternSegment === textSegment; // no wildcard: exact
    const prefix = patternSegment.slice(0, star);
    const suffix = patternSegment.slice(star + 1);
    return (
        textSegment.length >= prefix.length + suffix.length &&
        textSegment.startsWith(prefix) &&
        textSegment.endsWith(suffix)
    );
}

/**
 * Matches a path against a glob pattern. Validation guarantees at most one `**`,
 * so with no `**` the segments line up one-to-one, and with a `**` the segments
 * before it anchor to the start of the path and those after it to the end, with
 * `**` absorbing whatever is in between. No backtracking, so it can't blow up.
 */
export function matchGlob(pattern: string, path: string): boolean {
    const patternSegments = pattern.split("/");
    const pathSegments = path.split("/");
    const globstar = patternSegments.indexOf("**");

    if (globstar === -1) {
        return (
            patternSegments.length === pathSegments.length &&
            patternSegments.every((segment, i) => matchSegment(segment, pathSegments[i]))
        );
    }

    const before = patternSegments.slice(0, globstar);
    const after = patternSegments.slice(globstar + 1);
    if (pathSegments.length < before.length + after.length) return false;
    const tailOffset = pathSegments.length - after.length;
    return (
        before.every((segment, i) => matchSegment(segment, pathSegments[i])) &&
        after.every((segment, i) => matchSegment(segment, pathSegments[tailOffset + i]))
    );
}

/**
 * Rewrites a legacy regex route pattern as the equivalent glob, so a site
 * authored for the old regex matcher keeps the same reach once glob routing is
 * on. Under the regex a `*` became `.*` and crossed `/`, so:
 *  - a bare `*` matched everything, and becomes a root globstar `/**`;
 *  - a trailing `/` then `*` matched paths one or more levels below the prefix;
 *    the regex needed that slash, so it never matched the bare prefix. It
 *    becomes a globstar plus a required segment, keeping that reach without
 *    shadowing an exact route for the prefix itself.
 * A `*` in the middle of a pattern stays within its segment, and a pattern that
 * already uses `**` is returned unchanged.
 */
export function regexToGlobPattern(pattern: string): string {
    if (pattern.includes("**")) {
        return pattern; // already a glob pattern
    } else if (pattern === "*") {
        return "/**"; // bare catch-all matches everything
    } else if (pattern.endsWith("/*")) {
        // Require the slash plus a segment, so the catch-all matches strictly
        // below the prefix and never the bare prefix (as the regex did).
        return pattern.slice(0, -2) + "/**/*";
    } else {
        return pattern;
    }
}
