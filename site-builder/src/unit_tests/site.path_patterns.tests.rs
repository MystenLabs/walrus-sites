// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for `site::path_patterns`.
//!
//! The corpus mirrors `portal/common/lib/tests/path_patterns.test.ts` 1:1 and
//! then adds site-builder-specific cases (`narrows_under_glob`, portal-unseen
//! edges, and the write-boundary enforcement engine). Keep in sync with the
//! portal test corpus.

use super::*;
use crate::types::{Redirect, Redirects, RouteOps, Routes};

#[test]
fn test_count_stars() {
    let cases = vec![("", 0), ("/foo", 0), ("/foo/*", 1), ("/**", 2), ("a*b*", 2)];

    for (text, expected) in cases {
        assert_eq!(count_stars(text), expected, "count_stars({text:?})");
    }
}

#[test]
fn test_validate_glob_pattern_accepts() {
    let cases = vec![
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
    ];

    for pattern in cases {
        assert_eq!(
            validate_glob_pattern(pattern),
            Ok(()),
            "validate_glob_pattern({pattern:?})"
        );
    }
}

#[test]
fn test_validate_glob_pattern_rejects_per_segment() {
    let cases = vec![
        ("/foo/bar**", "bar**"),
        ("a*b*", "a*b*"),
        ("/x/a*a*a*/y", "a*a*a*"),
        ("***", "***"),
        ("/**bar**/x", "**bar**"),
        ("/*foo*", "*foo*"),
    ];

    for (pattern, segment) in cases {
        assert_eq!(
            validate_glob_pattern(pattern),
            Err(format!(
                r#"segment "{segment}" may use at most one '*', or be a whole-segment '**'"#
            )),
            "validate_glob_pattern({pattern:?})"
        );
    }
}

#[test]
fn test_validate_glob_pattern_rejects_globstars() {
    let cases = vec!["/foo/**/*/**/bar", "/a/**/b/**", "/**/x/**/y"];

    for pattern in cases {
        assert_eq!(
            validate_glob_pattern(pattern),
            Err("2 '**' globstars (max 1)".to_owned()),
            "validate_glob_pattern({pattern:?})"
        );
    }
}

#[test]
fn test_validate_regex_pattern_accepts() {
    let cases = vec!["/*", "/foo", "/foo/*", "/a/*/b/*", "a*b*", "/index.html"];

    for pattern in cases {
        assert_eq!(
            validate_regex_pattern(pattern),
            Ok(()),
            "validate_regex_pattern({pattern:?})"
        );
    }
}

#[test]
fn test_validate_regex_pattern_rejects_stars() {
    let cases = vec!["/a/*/b/*/c/*", "/a*b*c*", "***"];

    for pattern in cases {
        assert_eq!(
            validate_regex_pattern(pattern),
            Err("3 '*' characters (max 2 in total)".to_owned()),
            "validate_regex_pattern({pattern:?})"
        );
    }
}

#[test]
fn test_validate_regex_pattern_rejects_metachars() {
    let cases = vec![
        ("/foo(", '('),
        ("/foo)", ')'),
        ("/foo[a]", '['),
        ("/foo{2}", '{'),
        ("/a+b", '+'),
        ("/a?b", '?'),
        ("/a|b", '|'),
        ("/p$", '$'),
        ("/a\\b", '\\'),
    ];

    for (pattern, illegal) in cases {
        assert_eq!(
            validate_regex_pattern(pattern),
            Err(format!(r#"unsupported character "{illegal}""#)),
            "validate_regex_pattern({pattern:?})"
        );
    }
}

#[test]
fn test_validate_regex_pattern_rejects_every_illegal_char() {
    // The portal-mirrored cases above never surface '^', ']', or '}' as the
    // leftmost offender; sweep the whole set so dropping any char from
    // ILLEGAL_REGEX_CHARS fails a test.
    for c in ILLEGAL_REGEX_CHARS {
        assert_eq!(
            validate_regex_pattern(&format!("/p{c}q")),
            Err(format!(r#"unsupported character "{c}""#)),
            "validate_regex_pattern with {c:?}"
        );
    }
}

#[test]
fn test_rewrite_legacy_route_pattern() {
    let cases = vec![
        ("*", "/**"),
        ("/*", "/**/*"),
        ("/docs/*", "/docs/**/*"),
        // Mid-pattern, glued, and within-segment stars are deliberately
        // untouched; existing globs pass through unchanged.
        ("/forms/*/admin", "/forms/*/admin"),
        ("/blog/old-*", "/blog/old-*"),
        ("/foo/**", "/foo/**"),
        ("/about", "/about"),
    ];

    for (pattern, expected) in cases {
        assert_eq!(
            rewrite_legacy_route_pattern(pattern),
            expected,
            "rewrite_legacy_route_pattern({pattern:?})"
        );
    }
}

#[test]
fn test_narrows_under_glob() {
    let cases = vec![
        ("/api*", true),
        ("/forms/*/admin", true),
        ("/blog/old-*", true),
        ("*", false),
        ("/foo/*", false),
        ("/foo/**", false),
        ("", false),
        // Accepted gap: the narrowing middle star escapes via the
        // trailing-/* exclusion (deliberate; flagged to the migration owner).
        ("/a/*/b/*", false),
    ];

    for (pattern, expected) in cases {
        assert_eq!(
            narrows_under_glob(pattern),
            expected,
            "narrows_under_glob({pattern:?})"
        );
    }
}

/// Edge cases the portal test corpus does not cover: pin exact behavior (and
/// no panics) so it cannot drift silently.
#[test]
fn test_portal_unseen_edges() {
    // The empty pattern is valid under both validators and rewrites unchanged.
    assert_eq!(validate_glob_pattern(""), Ok(()));
    assert_eq!(validate_regex_pattern(""), Ok(()));
    assert_eq!(rewrite_legacy_route_pattern(""), "");

    // A bare globstar is a single whole-segment `**`: glob-valid.
    assert_eq!(validate_glob_pattern("**"), Ok(()));

    // No leading slash: the trailing-/* rewrite still applies.
    assert_eq!(rewrite_legacy_route_pattern("foo/*"), "foo/**/*");

    // Empty segments from a double slash are fine (zero stars).
    assert_eq!(validate_glob_pattern("/foo//bar"), Ok(()));

    // Unicode: stars are counted per code point, not per byte.
    assert_eq!(count_stars("日*本*"), 2);
    assert_eq!(rewrite_legacy_route_pattern("/café/*"), "/café/**/*");
    assert_eq!(
        validate_glob_pattern("/日*本*/x"),
        Err(r#"segment "日*本*" may use at most one '*', or be a whole-segment '**'"#.to_owned())
    );
}

// Write-boundary enforcement (site-builder only; not mirrored in the portal).

fn make_routes(pairs: &[(&str, &str)]) -> Routes {
    Routes(
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    )
}

fn make_redirects(patterns: &[&str]) -> Redirects {
    Redirects(
        patterns
            .iter()
            .map(|p| {
                (
                    (*p).to_owned(),
                    Redirect {
                        location: "/target".to_owned(),
                        status_code: 301,
                    },
                )
            })
            .collect(),
    )
}

fn expect_glob_invalid(target: PatternTarget, pattern: &str, segment: &str) -> PatternIssue {
    PatternIssue {
        target,
        pattern: pattern.to_owned(),
        reason: format!(
            r#"segment "{segment}" may use at most one '*', or be a whole-segment '**'"#
        ),
        kind: PatternIssueKind::GlobInvalid,
    }
}

#[test]
fn test_apply_route_rewrites_rewrites_keys_and_reports_changes() {
    let mut routes = make_routes(&[
        ("/docs/*", "/index.html"),
        ("/about", "/about.html"),
        ("*", "/404.html"),
    ]);

    let changed = apply_route_rewrites(&mut routes).expect("no collisions");

    // Only the pairs that actually changed, sorted by original.
    assert_eq!(
        changed,
        vec![
            ("*".to_owned(), "/**".to_owned()),
            ("/docs/*".to_owned(), "/docs/**/*".to_owned()),
        ]
    );
    // Keys are rewritten in place; each value stays with its original entry.
    assert_eq!(
        routes,
        make_routes(&[
            ("/docs/**/*", "/index.html"),
            ("/about", "/about.html"),
            ("/**", "/404.html"),
        ])
    );
}

#[test]
fn test_apply_route_rewrites_collision_leaves_routes_unchanged() {
    // Colliding keys with DIFFERENT values: merging would drop a route.
    let mut routes = make_routes(&[("/docs/*", "/a.html"), ("/docs/**/*", "/b.html")]);
    let before = routes.clone();

    let collisions = apply_route_rewrites(&mut routes).expect_err("colliding rewrites");

    assert_eq!(
        collisions,
        vec![RewriteCollision {
            rewritten: "/docs/**/*".to_owned(),
            originals: vec!["/docs/*".to_owned(), "/docs/**/*".to_owned()],
        }]
    );
    assert_eq!(routes, before, "a failed rewrite must not mutate the input");
}

#[test]
fn test_apply_route_rewrites_merges_identical_value_collisions() {
    // Colliding keys with the SAME value lose nothing: merge instead of failing.
    let mut routes = make_routes(&[
        ("/docs/*", "/a.html"),
        ("/docs/**/*", "/a.html"),
        ("/other", "/b.html"),
    ]);

    let changed = apply_route_rewrites(&mut routes).expect("identical values must merge");

    assert_eq!(
        changed,
        vec![("/docs/*".to_owned(), "/docs/**/*".to_owned())]
    );
    assert_eq!(
        routes,
        make_routes(&[("/docs/**/*", "/a.html"), ("/other", "/b.html")])
    );
}

#[test]
fn test_rewrite_session_applies_reports_and_restores() {
    let mut routes = Some(make_routes(&[("/docs/*", "/a.html"), ("/fine", "/b.html")]));

    let session = RewriteSession::apply(&mut routes).expect("no collisions");
    assert_eq!(
        session.pairs(),
        &[("/docs/*".to_owned(), "/docs/**/*".to_owned())]
    );
    assert_eq!(
        routes,
        Some(make_routes(&[
            ("/docs/**/*", "/a.html"),
            ("/fine", "/b.html")
        ]))
    );

    session.restore(&mut routes);
    assert_eq!(
        routes,
        Some(make_routes(&[("/docs/*", "/a.html"), ("/fine", "/b.html")]))
    );
}

#[test]
fn test_rewrite_session_empty_cases() {
    // Explicitly empty session (flag off): no pairs.
    assert!(RewriteSession::empty().pairs().is_empty());

    // No routes declared: empty session, restore is a no-op.
    let mut routes: Option<Routes> = None;
    let session = RewriteSession::apply(&mut routes).expect("no routes, no collisions");
    assert!(session.pairs().is_empty());
    session.restore(&mut routes);
    assert_eq!(routes, None);

    // Routes present but nothing to rewrite: no pairs, restore keeps the set.
    let mut routes = Some(make_routes(&[("/docs/**", "/a.html")]));
    let before = routes.clone();
    let session = RewriteSession::apply(&mut routes).expect("no collisions");
    assert!(session.pairs().is_empty());
    session.restore(&mut routes);
    assert_eq!(routes, before);

    // Value-losing collisions propagate.
    let mut routes = Some(make_routes(&[
        ("/docs/*", "/a.html"),
        ("/docs/**/*", "/b.html"),
    ]));
    assert!(RewriteSession::apply(&mut routes).is_err());
}

/// THE key regression for `--rewrite-legacy-routes`: rewriting the local
/// legacy-spelled routes into the on-chain glob spelling must make the diff
/// idempotent (`Unchanged`), while the raw spelling would replace the set on
/// every deploy.
#[test]
fn test_apply_route_rewrites_then_diff_is_idempotent() {
    let existing = make_routes(&[("/docs/**/*", "/index.html"), ("/**", "/404.html")]);

    let mut local = make_routes(&[("/docs/*", "/index.html"), ("*", "/404.html")]);
    apply_route_rewrites(&mut local).expect("no collisions");
    assert!(matches!(
        Routes::diff_opt(Some(&local), Some(&existing)),
        RouteOps::Unchanged
    ));

    let raw_local = make_routes(&[("/docs/*", "/index.html"), ("*", "/404.html")]);
    assert!(matches!(
        Routes::diff_opt(Some(&raw_local), Some(&existing)),
        RouteOps::Replace(_)
    ));
}

#[test]
fn test_check_write_boundary_replace_collects_all_offenders() {
    let local = make_routes(&[("a*b*", "/v1"), ("/x/a*a*a*/y", "/v2"), ("/fine", "/v3")]);

    let report = check_write_boundary(Some(&local), None, None, None, &[]);

    // Both offenders, in key order, each with its exact validator reason.
    assert_eq!(
        report.errors,
        vec![
            expect_glob_invalid(PatternTarget::Route, "/x/a*a*a*/y", "a*a*a*"),
            expect_glob_invalid(PatternTarget::Route, "a*b*", "a*b*"),
        ]
    );
    assert!(report.warnings.is_empty(), "'/fine' must produce nothing");
}

#[test]
fn test_check_write_boundary_unchanged_invalid_is_warning() {
    let set = make_routes(&[("a*b*", "/v")]);

    let report = check_write_boundary(Some(&set), Some(&set), None, None, &[]);

    assert!(
        report.errors.is_empty(),
        "nothing is written; deploy proceeds"
    );
    assert_eq!(
        report.warnings,
        vec![expect_glob_invalid(PatternTarget::Route, "a*b*", "a*b*")]
    );
}

#[test]
fn test_check_write_boundary_empty_and_none_sets() {
    // The existing on-chain set is deliberately invalid: only the set about
    // to be written is validated, never the one being removed/replaced.
    let on_chain = make_routes(&[("a*b*", "/v")]);
    let empty = Routes::empty();

    let cases: Vec<(Option<&Routes>, Option<&Routes>)> = vec![
        // Removal is a Replace with the empty set: nothing to validate.
        (None, Some(&on_chain)),
        (None, None),
        // Replacing with an explicitly empty set validates nothing either.
        (Some(&empty), Some(&on_chain)),
    ];

    for (local, existing) in cases {
        let report = check_write_boundary(local, existing, None, None, &[]);
        assert!(
            report.errors.is_empty() && report.warnings.is_empty(),
            "expected empty report for local={local:?}, existing={existing:?}"
        );
    }
}

#[test]
fn test_check_write_boundary_fresh_publish_validates() {
    let local = make_routes(&[("***", "/v")]);

    let report = check_write_boundary(Some(&local), None, None, None, &[]);

    assert_eq!(
        report.errors,
        vec![expect_glob_invalid(PatternTarget::Route, "***", "***")]
    );
    assert!(report.warnings.is_empty());
}

#[test]
fn test_check_write_boundary_redirects_glob_structural_only() {
    // Regex metacharacters are glob literals: no issue of any kind.
    let parens = make_redirects(&["/wiki/(x)"]);
    let report = check_write_boundary(None, None, Some(&parens), None, &[]);
    assert!(report.errors.is_empty() && report.warnings.is_empty());

    // Structurally invalid: an error when written, a warning when unchanged.
    let invalid = make_redirects(&["/a/**/b/**"]);
    let issue = PatternIssue {
        target: PatternTarget::Redirect,
        pattern: "/a/**/b/**".to_owned(),
        reason: "2 '**' globstars (max 1)".to_owned(),
        kind: PatternIssueKind::GlobInvalid,
    };

    let written = check_write_boundary(None, None, Some(&invalid), None, &[]);
    assert_eq!(written.errors, vec![issue.clone()]);
    assert!(written.warnings.is_empty());

    let unchanged = check_write_boundary(None, None, Some(&invalid), Some(&invalid), &[]);
    assert!(unchanged.errors.is_empty());
    assert_eq!(unchanged.warnings, vec![issue]);

    // A pattern that takes legacy/narrowing warnings as a route takes none
    // as a redirect, written or unchanged.
    let star = make_redirects(&["/api*"]);
    for existing in [None, Some(&star)] {
        let report = check_write_boundary(None, None, Some(&star), existing, &[]);
        assert!(
            report.errors.is_empty() && report.warnings.is_empty(),
            "existing={existing:?}"
        );
    }
}

#[test]
fn test_check_write_boundary_legacy_and_narrowing_warnings() {
    let local = make_routes(&[("/a/*/b/**", "/v1"), ("/api*", "/v2"), ("/foo/*", "/v3")]);

    let report = check_write_boundary(Some(&local), None, None, None, &[]);

    assert!(report.errors.is_empty());
    assert_eq!(
        report.warnings,
        vec![
            // Glob-valid but over the legacy star cap: skipped by legacy
            // portals (no rewrite involved, so not a sequencing hazard).
            PatternIssue {
                target: PatternTarget::Route,
                pattern: "/a/*/b/**".to_owned(),
                reason: "3 '*' characters (max 2 in total)".to_owned(),
                kind: PatternIssueKind::LegacySkipped,
            },
            // StarNarrowing carries no validator reason: pinned empty.
            PatternIssue {
                target: PatternTarget::Route,
                pattern: "/api*".to_owned(),
                reason: String::new(),
                kind: PatternIssueKind::StarNarrowing,
            },
        ]
    );
}

#[test]
fn test_check_write_boundary_rewrite_sequencing_hazard() {
    let local = make_routes(&[("/docs/**/*", "/index.html"), ("/**", "/404.html")]);
    let rewritten = [
        ("/docs/*".to_owned(), "/docs/**/*".to_owned()),
        ("*".to_owned(), "/**".to_owned()),
    ];

    let report = check_write_boundary(Some(&local), None, None, None, &rewritten);

    // `/**` is 2 stars = legacy-valid, so rewriting `*` is hazard-free; only
    // the 3-star rewrite output `/docs/**/*` trips the sequencing hazard.
    assert!(report.errors.is_empty());
    assert_eq!(
        report.warnings,
        vec![PatternIssue {
            target: PatternTarget::Route,
            pattern: "/docs/**/*".to_owned(),
            reason: "3 '*' characters (max 2 in total)".to_owned(),
            kind: PatternIssueKind::LegacyAfterRewrite,
        }]
    );

    // The rendered hazard names both the original and the stored spelling.
    assert_eq!(
        format_warning(&report.warnings[0], &rewritten),
        concat!(
            "'/docs/*' was rewritten to '/docs/**/*', which portals still running legacy ",
            "(regex) routing cannot match (3 '*' characters (max 2 in total)) and will skip. ",
            "Only deploy with --rewrite-legacy-routes once portal deployments run glob routing.",
        )
    );
}

#[test]
fn test_check_write_boundary_glob_invalid_suppresses_secondary_warnings() {
    // Were `a*b*` glob-valid it would also be flagged as narrowing; the
    // glob-structural failure must be the only issue reported for it.
    let set = make_routes(&[("a*b*", "/v")]);

    let report = check_write_boundary(Some(&set), Some(&set), None, None, &[]);

    assert!(report.errors.is_empty());
    assert_eq!(report.warnings.len(), 1, "exactly one issue in total");
    assert_eq!(report.warnings[0].kind, PatternIssueKind::GlobInvalid);
}

#[test]
fn test_format_errors_text_shape() {
    let errors = vec![
        expect_glob_invalid(PatternTarget::Route, "a*b*", "a*b*"),
        expect_glob_invalid(PatternTarget::Route, "/x/a*a*a*/y", "a*a*a*"),
        PatternIssue {
            target: PatternTarget::Redirect,
            pattern: "/a/**/b/**".to_owned(),
            reason: "2 '**' globstars (max 1)".to_owned(),
            kind: PatternIssueKind::GlobInvalid,
        },
    ];

    let expected = concat!(
        "found 3 invalid route/redirect pattern(s); portals cannot match them, ",
        "refusing to store them on-chain:\n",
        "  - route pattern 'a*b*': segment \"a*b*\" may use at most one '*', ",
        "or be a whole-segment '**'\n",
        "  - route pattern '/x/a*a*a*/y': segment \"a*a*a*\" may use at most one '*', ",
        "or be a whole-segment '**'\n",
        "  - redirect pattern '/a/**/b/**': 2 '**' globstars (max 1)\n",
        "Routes and redirects are stored as complete sets, so every pattern in ",
        "ws-resources.json must be valid before this deploy can proceed — including ",
        "patterns you did not change.",
    );

    assert_eq!(format_errors(&errors, &[]), expected);
}

#[test]
fn test_format_errors_names_original_spelling_for_rewritten_patterns() {
    // A pattern can be glob-invalid AND changed by the rewrite (an invalid
    // segment before a trailing `/*`): the error must point at the spelling
    // that actually exists in ws-resources.json, not the rewritten one.
    let errors = vec![expect_glob_invalid(
        PatternTarget::Route,
        "/a*b*/**/*",
        "a*b*",
    )];
    let rewritten = [("/a*b*/*".to_owned(), "/a*b*/**/*".to_owned())];

    let expected = concat!(
        "found 1 invalid route/redirect pattern(s); portals cannot match them, ",
        "refusing to store them on-chain:\n",
        "  - route pattern '/a*b*/*' (stored as '/a*b*/**/*' by --rewrite-legacy-routes): ",
        "segment \"a*b*\" may use at most one '*', or be a whole-segment '**'\n",
        "Routes and redirects are stored as complete sets, so every pattern in ",
        "ws-resources.json must be valid before this deploy can proceed — including ",
        "patterns you did not change.",
    );

    assert_eq!(format_errors(&errors, &rewritten), expected);
}

/// Pins the remaining user-facing wordings byte-exact (the hazard text is
/// pinned in the sequencing-hazard test, `format_errors` in the text-shape
/// test): the message builders' strings are settled and must not drift.
#[test]
fn test_format_warning_and_rewrite_texts() {
    assert_eq!(
        format_warning(
            &expect_glob_invalid(PatternTarget::Route, "a*b*", "a*b*"),
            &[]
        ),
        concat!(
            "On-chain route pattern 'a*b*' is invalid (segment \"a*b*\" may use at most ",
            "one '*', or be a whole-segment '**'); portals skip it when matching. This deploy ",
            "leaves routes unchanged and can proceed, but fix the pattern in ",
            "ws-resources.json: the next deploy that modifies routes will refuse to store it.",
        )
    );

    let glob_invalid_redirect = PatternIssue {
        target: PatternTarget::Redirect,
        pattern: "/a/**/b/**".to_owned(),
        reason: "2 '**' globstars (max 1)".to_owned(),
        kind: PatternIssueKind::GlobInvalid,
    };
    assert_eq!(
        format_warning(&glob_invalid_redirect, &[]),
        concat!(
            "On-chain redirect pattern '/a/**/b/**' is invalid (2 '**' globstars (max 1)); ",
            "portals skip it when matching. This deploy leaves redirects unchanged and can ",
            "proceed, but fix the pattern in ws-resources.json: the next deploy that modifies ",
            "redirects will refuse to store it.",
        )
    );

    let legacy_skipped = PatternIssue {
        target: PatternTarget::Route,
        pattern: "/a/*/b/**".to_owned(),
        reason: "3 '*' characters (max 2 in total)".to_owned(),
        kind: PatternIssueKind::LegacySkipped,
    };
    assert_eq!(
        format_warning(&legacy_skipped, &[]),
        concat!(
            "Route pattern '/a/*/b/**' is valid glob syntax, but exceeds the legacy matcher's ",
            "limits (3 '*' characters (max 2 in total)); portals still running legacy (regex) ",
            "routing skip it. It only takes effect on portals with glob routing enabled.",
        )
    );

    let narrowing = PatternIssue {
        target: PatternTarget::Route,
        pattern: "/api*".to_owned(),
        reason: String::new(),
        kind: PatternIssueKind::StarNarrowing,
    };
    assert_eq!(
        format_warning(&narrowing, &[]),
        concat!(
            "Note on route pattern '/api*': under glob routing, '*' matches within a single ",
            "path segment (legacy regex routing let it cross '/'). The pattern is valid and ",
            "stored as-is; if you want it to match everything below a prefix, use a ",
            "whole-segment '**' — e.g. '/foo/**' matches '/foo' and everything under it.",
        )
    );

    let rewritten = [
        ("*".to_owned(), "/**".to_owned()),
        ("/docs/*".to_owned(), "/docs/**/*".to_owned()),
    ];
    assert_eq!(
        format_rewrite_notice(&rewritten),
        concat!(
            "--rewrite-legacy-routes: storing 2 route pattern(s) in glob form ",
            "(ws-resources.json is not modified):\n",
            "  - '*' -> '/**'\n",
            "  - '/docs/*' -> '/docs/**/*'",
        )
    );

    let collisions = [
        RewriteCollision {
            rewritten: "/**".to_owned(),
            originals: vec!["*".to_owned(), "/**".to_owned()],
        },
        RewriteCollision {
            rewritten: "/docs/**/*".to_owned(),
            originals: vec!["/docs/*".to_owned(), "/docs/**/*".to_owned()],
        },
    ];
    assert_eq!(
        format_rewrite_collisions(&collisions),
        concat!(
            "--rewrite-legacy-routes produced conflicting route patterns: '*' and '/**' both ",
            "rewrite to '/**'. Remove or merge these entries in ws-resources.json.\n",
            "--rewrite-legacy-routes produced conflicting route patterns: '/docs/*' and ",
            "'/docs/**/*' both rewrite to '/docs/**/*'. Remove or merge these entries in ",
            "ws-resources.json.",
        )
    );
}
