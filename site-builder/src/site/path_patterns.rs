// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Validating and rewriting owner-supplied route/redirect patterns.
//!
//! This is a deliberate 1:1 port of the validation and rewrite rules in
//! `portal/common/lib/src/path_patterns.ts` (introduced in PR #720); the
//! portal's matching functions (`matchGlob` et al.) are NOT ported, since the
//! site-builder never matches paths — it only validates patterns at deploy
//! time.
//!
//! KEEP IN SYNC: any change to the portal's validation rules, reason-string
//! wordings, or rewrite table must be mirrored here, and vice versa. The
//! reason strings returned by the validators must match the portal's
//! byte-for-byte.
//!
//! Grammar: `*` matches within one path segment, a whole-segment `**` matches
//! across segments, and every other character is a literal.
//!
//! Wildcard caps keep matching cheap: glob patterns (redirects always; routes
//! when the flag is on) allow one `*` per segment and one `**` per pattern, so
//! the portal's `matchGlob` never backtracks. The legacy regex branch (routes,
//! flag off) allows only path characters plus `*` and `.`, capping the
//! compiled regex at two `.*`s — worst case quadratic, a bounded stall rather
//! than a freeze. Rejected regex metacharacters are plain literals under the
//! glob matcher.

use std::collections::BTreeMap;

use crate::types::{RedirectOps, Redirects, RouteOps, Routes};

#[cfg(test)]
#[path = "../unit_tests/site.path_patterns.tests.rs"]
mod path_patterns_tests;

/// Max total `*` characters in a legacy-regex pattern (glob is bounded per segment).
pub const MAX_STARS: usize = 2;

/// Max whole-segment `**` globstars in a glob pattern (each adds backtracking).
pub const MAX_GLOBSTARS: usize = 1;

/// Regex metacharacters rejected in legacy-regex route patterns. Only `*` (the
/// wildcard, translated to `.*`) and `.` (today's any-char) are allowed through.
const ILLEGAL_REGEX_CHARS: [char; 12] =
    ['(', ')', '[', ']', '{', '}', '+', '?', '^', '$', '|', '\\'];

/// Number of `*` characters in `text`. Iterates chars (code points), not bytes.
pub fn count_stars(text: &str) -> usize {
    text.chars().filter(|&c| c == '*').count()
}

/// Validates a glob route/redirect pattern. Each segment may use at most one
/// `*` or be a whole-segment `**`, and a pattern may have at most one `**`.
/// Callers skip (and report) patterns that fail rather than feeding them to
/// the matcher.
///
/// `Err(reason)` mirrors the portal's `PatternValidation` reason strings
/// exactly.
pub fn validate_glob_pattern(pattern: &str) -> Result<(), String> {
    let mut globstars = 0;
    for segment in pattern.split('/') {
        if segment == "**" {
            globstars += 1;
            continue;
        }
        // Any other segment may use at most one `*` (so `bar**`, `a*b*`, `***`
        // are rejected: `**` is only valid as a whole segment).
        if count_stars(segment) > 1 {
            return Err(format!(
                r#"segment "{segment}" may use at most one '*', or be a whole-segment '**'"#
            ));
        }
    }
    if globstars > MAX_GLOBSTARS {
        return Err(format!("{globstars} '**' globstars (max {MAX_GLOBSTARS})"));
    }
    Ok(())
}

/// Validates a legacy-regex route pattern (the matcher used while the glob
/// flag is off). Rejects regex metacharacters and caps the total wildcard
/// count. Callers skip (and report) patterns that fail rather than feeding
/// them to the matcher.
///
/// `Err(reason)` mirrors the portal's `PatternValidation` reason strings
/// exactly.
pub fn validate_regex_pattern(pattern: &str) -> Result<(), String> {
    if let Some(illegal) = pattern.chars().find(|c| ILLEGAL_REGEX_CHARS.contains(c)) {
        return Err(format!(r#"unsupported character "{illegal}""#));
    }
    let total = count_stars(pattern);
    if total > MAX_STARS {
        return Err(format!("{total} '*' characters (max {MAX_STARS} in total)"));
    }
    Ok(())
}

/// Rewrites a legacy regex route pattern as the equivalent glob, so a site
/// authored for the old regex matcher keeps the same reach once glob routing
/// is on. Under the regex a `*` became `.*` and crossed `/`, so:
///  - a bare `*` matched everything, and becomes a root globstar `/**`;
///  - a trailing `/` then `*` matched paths one or more levels below the
///    prefix; the regex needed that slash, so it never matched the bare
///    prefix. It becomes a globstar plus a required segment, keeping that
///    reach without shadowing an exact route for the prefix itself.
///
/// A `*` in the middle of a pattern stays within its segment, and a pattern
/// that already uses `**` is returned unchanged.
pub fn rewrite_legacy_route_pattern(pattern: &str) -> String {
    if pattern.contains("**") {
        pattern.to_owned() // already a glob pattern
    } else if pattern == "*" {
        "/**".to_owned() // bare catch-all matches everything
    } else if let Some(prefix) = pattern.strip_suffix("/*") {
        // Require the slash plus a segment, so the catch-all matches strictly
        // below the prefix and never the bare prefix (as the regex did).
        format!("{prefix}/**/*")
    } else {
        pattern.to_owned()
    }
}

/// Site-builder-specific (not in the portal): true when a pattern's reach
/// narrows under glob matching and no rewrite will widen it back — it has a
/// `*` that under the legacy regex crossed `/` boundaries, but is neither the
/// bare catch-all nor a trailing `/*` (both of which
/// [`rewrite_legacy_route_pattern`] widens back), and is not already a glob.
///
/// Known accepted gap: a pattern like `/a/*/b/*` has a narrowing middle star
/// but escapes via the trailing-`/*` exclusion (deliberate, pinned by test).
// TODO(sew-1001): remove with the routing migration.
pub fn narrows_under_glob(pattern: &str) -> bool {
    count_stars(pattern) > 0
        && pattern != "*"
        && !pattern.ends_with("/*")
        && !pattern.contains("**")
}

/// Which pattern set a [`PatternIssue`] was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternTarget {
    /// A route pattern (a key of the site's routes).
    Route,
    /// A redirect pattern (a key of the site's redirects).
    Redirect,
}

impl PatternTarget {
    /// Singular noun for user-facing messages.
    fn noun(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::Redirect => "redirect",
        }
    }

    /// Plural noun for user-facing messages.
    fn plural(self) -> &'static str {
        match self {
            Self::Route => "routes",
            Self::Redirect => "redirects",
        }
    }
}

/// What is wrong (or noteworthy) about a pattern at the write boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternIssueKind {
    /// Not valid glob syntax: portals cannot match it. A hard error when the
    /// pattern's set is about to be written; a warning when the set stays
    /// unchanged on-chain.
    GlobInvalid,
    /// Valid glob, but beyond the legacy (regex) matcher's limits: portals
    /// with the glob flag off skip it, so it only takes effect once the fleet
    /// runs glob routing.
    LegacySkipped,
    /// A pattern our own `--rewrite-legacy-routes` rewrite produced that
    /// legacy portals cannot match: deploying it before the fleet runs glob
    /// routing is a sequencing hazard.
    // TODO(sew-1001): remove with the routing migration.
    LegacyAfterRewrite,
    /// Informational: the pattern's `*` crossed `/` under the legacy regex
    /// matcher but stays within one segment under glob, narrowing its reach.
    // TODO(sew-1001): remove with the routing migration.
    StarNarrowing,
}

/// One validation finding for a single route/redirect pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternIssue {
    /// Which set the pattern belongs to.
    pub target: PatternTarget,
    /// The pattern as it appears in the (to-be-written or on-chain) set.
    pub pattern: String,
    /// The validator's reason string; empty for
    /// [`PatternIssueKind::StarNarrowing`], whose user text is produced
    /// entirely by [`format_warning`].
    pub reason: String,
    /// What is wrong (or noteworthy) about the pattern.
    pub kind: PatternIssueKind,
}

/// Everything [`check_write_boundary`] found: `errors` must block the deploy,
/// `warnings` must not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteBoundaryReport {
    /// Structurally invalid patterns in a set about to be written on-chain.
    pub errors: Vec<PatternIssue>,
    /// Non-blocking findings: invalid-but-unchanged patterns, legacy-portal
    /// skips, rewrite sequencing hazards, and narrowing notes.
    pub warnings: Vec<PatternIssue>,
}

/// Distinct route patterns that [`apply_route_rewrites`] would collapse into
/// the same stored key (which would silently drop all but one value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteCollision {
    /// The contested rewritten key.
    pub rewritten: String,
    /// The original patterns (sorted) that all rewrite to `rewritten`.
    pub originals: Vec<String>,
}

/// Validates the route and redirect patterns of a deploy at the write
/// boundary, i.e. against what the deploy actually stores on-chain.
///
/// Written-vs-unchanged is decided by [`Routes::diff_opt`] and
/// [`Redirects::diff_opt`] — the same single source of truth the deploy's
/// site diff uses — so this check cannot diverge from what is written. Sets
/// are stored replace-all, so when a set changes, every key of the complete
/// new set is validated (including patterns this deploy did not touch);
/// structurally invalid ones are hard [`WriteBoundaryReport::errors`]. When a
/// locally-declared set is unchanged, the same checks run but invalid
/// patterns are only warnings: nothing new lands on-chain, so the deploy may
/// proceed.
///
/// Routes additionally take the migration checks (legacy-portal skips,
/// rewrite sequencing hazards, `*`-narrowing notes); redirects are
/// glob-structural only, since they never went through the legacy regex
/// matcher.
///
/// `local_routes` is the post-rewrite set when `--rewrite-legacy-routes` is
/// on (raw otherwise), and `rewritten` holds the `(original, stored)` pairs
/// the rewrite actually changed (empty when the flag is off).
pub fn check_write_boundary(
    local_routes: Option<&Routes>,
    existing_routes: Option<&Routes>,
    local_redirects: Option<&Redirects>,
    existing_redirects: Option<&Redirects>,
    rewritten: &[(String, String)],
) -> WriteBoundaryReport {
    let mut report = WriteBoundaryReport::default();

    match Routes::diff_opt(local_routes, existing_routes) {
        RouteOps::Replace(written) => {
            classify_written(route_pattern_issues(&written, rewritten), &mut report)
        }
        // Nothing new lands on-chain, so nothing can block: every finding in
        // a locally-declared-but-unchanged set is a warning.
        RouteOps::Unchanged => {
            if let Some(local) = local_routes {
                report
                    .warnings
                    .extend(route_pattern_issues(local, rewritten));
            }
        }
    }

    match Redirects::diff_opt(local_redirects, existing_redirects) {
        RedirectOps::Replace(written) => {
            classify_written(redirect_pattern_issues(&written), &mut report)
        }
        RedirectOps::Unchanged => {
            if let Some(local) = local_redirects {
                report.warnings.extend(redirect_pattern_issues(local));
            }
        }
    }

    report
}

/// Classifies the findings of a set that is about to be written on-chain: a
/// structurally invalid glob pattern blocks the deploy, every other finding
/// (legacy-portal skips, rewrite sequencing hazards, narrowing notes) warns.
/// Together with the warnings-only `Unchanged` arms in
/// [`check_write_boundary`], this is the validator's entire
/// blocking-vs-non-blocking posture: flipping any class is a change here
/// alone.
fn classify_written(issues: Vec<PatternIssue>, report: &mut WriteBoundaryReport) {
    for issue in issues {
        if issue.kind == PatternIssueKind::GlobInvalid {
            report.errors.push(issue);
        } else {
            report.warnings.push(issue);
        }
    }
}

/// Every finding for a route set, in key order: the glob-structural check,
/// then — for glob-valid patterns only — the migration checks.
fn route_pattern_issues(set: &Routes, rewritten: &[(String, String)]) -> Vec<PatternIssue> {
    let mut issues = Vec::new();
    for (pattern, _) in set.0.iter() {
        match glob_invalid_issue(PatternTarget::Route, pattern) {
            // A glob-invalid pattern produces exactly this one issue; the
            // migration checks only apply to glob-valid patterns.
            Some(issue) => issues.push(issue),
            None => issues.extend(route_migration_issue(pattern, rewritten)),
        }
    }
    issues
}

/// Every finding for a redirect set, in key order: glob-structural only —
/// redirects never took the legacy regex path, so no migration checks apply.
fn redirect_pattern_issues(set: &Redirects) -> Vec<PatternIssue> {
    set.0
        .iter()
        .filter_map(|(pattern, _)| glob_invalid_issue(PatternTarget::Redirect, pattern))
        .collect()
}

/// The glob-structural check shared by routes and redirects.
fn glob_invalid_issue(target: PatternTarget, pattern: &str) -> Option<PatternIssue> {
    validate_glob_pattern(pattern)
        .err()
        .map(|reason| PatternIssue {
            target,
            pattern: pattern.to_owned(),
            reason,
            kind: PatternIssueKind::GlobInvalid,
        })
}

/// The migration checks for a glob-valid route pattern: skipped by the legacy
/// matcher (a sequencing hazard when the pattern is one of our own rewrite
/// outputs), otherwise the `*`-narrowing note.
// TODO(sew-1001): remove with the routing migration.
fn route_migration_issue(pattern: &str, rewritten: &[(String, String)]) -> Option<PatternIssue> {
    match validate_regex_pattern(pattern) {
        Err(reason) => {
            let kind = if rewritten.iter().any(|(_, stored)| stored == pattern) {
                PatternIssueKind::LegacyAfterRewrite
            } else {
                PatternIssueKind::LegacySkipped
            };
            Some(PatternIssue {
                target: PatternTarget::Route,
                pattern: pattern.to_owned(),
                reason,
                kind,
            })
        }
        Ok(()) => narrows_under_glob(pattern).then(|| PatternIssue {
            target: PatternTarget::Route,
            pattern: pattern.to_owned(),
            reason: String::new(),
            kind: PatternIssueKind::StarNarrowing,
        }),
    }
}

/// Rewrites every route key through [`rewrite_legacy_route_pattern`], in
/// place, keeping each value with its key.
///
/// Returns the `(original, stored)` pairs that actually changed, sorted by
/// original. If two or more distinct originals rewrite to the same key (e.g.
/// `/docs/*` and `/docs/**/*` both become `/docs/**/*`), their entries merge
/// into one when the values are identical (nothing is lost); when the values
/// differ, picking one would silently drop a route, so `routes` is left
/// unchanged and all such collisions are returned.
pub fn apply_route_rewrites(
    routes: &mut Routes,
) -> Result<Vec<(String, String)>, Vec<RewriteCollision>> {
    // Detect collisions up front, so a failed rewrite leaves `routes` untouched.
    let mut entries_by_stored: BTreeMap<String, Vec<(&String, &String)>> = BTreeMap::new();
    for (original, value) in routes.0.iter() {
        entries_by_stored
            .entry(rewrite_legacy_route_pattern(original))
            .or_default()
            .push((original, value));
    }
    let collisions: Vec<RewriteCollision> = entries_by_stored
        .into_iter()
        .filter(|(_, entries)| {
            entries.len() > 1 && entries.iter().any(|&(_, value)| value != entries[0].1)
        })
        .map(|(rewritten, entries)| RewriteCollision {
            rewritten,
            // Already sorted: pushed in the route map's key order.
            originals: entries
                .into_iter()
                .map(|(original, _)| original.clone())
                .collect(),
        })
        .collect();
    if !collisions.is_empty() {
        return Err(collisions);
    }

    // Rebuild the map under the rewritten keys; identical-value collisions
    // merge into a single entry. Record the changed pairs (sorted by
    // original, as the map iterates in key order).
    let mut changed = Vec::new();
    routes.0.map_keys(|original| {
        let stored = rewrite_legacy_route_pattern(original);
        if stored != *original {
            changed.push((original.clone(), stored.clone()));
        }
        stored
    });
    Ok(changed)
}

/// One application of `--rewrite-legacy-routes`: applies the rewrite and
/// holds everything needed to report it (the changed pairs) and to undo it in
/// memory (the pre-rewrite routes) before the configuration file is
/// persisted.
// TODO(sew-1001): remove with the routing migration.
#[derive(Debug, Default)]
pub struct RewriteSession {
    original_routes: Option<Routes>,
    pairs: Vec<(String, String)>,
}

impl RewriteSession {
    /// A session that rewrote nothing (flag off, or no routes declared).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Applies [`apply_route_rewrites`] to `routes`, capturing the original
    /// set (only when something changed) and the changed pairs.
    pub fn apply(routes: &mut Option<Routes>) -> Result<Self, Vec<RewriteCollision>> {
        let Some(routes) = routes.as_mut() else {
            return Ok(Self::empty());
        };
        let original = routes.clone();
        let pairs = apply_route_rewrites(routes)?;
        Ok(Self {
            original_routes: (!pairs.is_empty()).then_some(original),
            pairs,
        })
    }

    /// The `(original, stored)` pairs the rewrite changed, sorted by original.
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    /// Restores the pre-rewrite routes. The rewrite is in-memory only, and
    /// persisting the configuration file writes the whole `WSResources` back
    /// to disk, so this must run before that write. Consumes the session:
    /// there is nothing left to restore twice.
    pub fn restore(self, routes: &mut Option<Routes>) {
        if let Some(original) = self.original_routes {
            *routes = Some(original);
        }
    }
}

/// One blocking message covering every structurally invalid pattern in a set
/// about to be written. `rewritten` (the `(original, stored)` pairs from
/// [`apply_route_rewrites`]) recovers the original spelling for route patterns
/// the rewrite changed, so the error points at what is actually in
/// ws-resources.json.
pub fn format_errors(errors: &[PatternIssue], rewritten: &[(String, String)]) -> String {
    let mut message = format!(
        "found {} invalid route/redirect pattern(s); portals cannot match them, refusing to \
         store them on-chain:\n",
        errors.len()
    );
    for issue in errors {
        // Redirects are never rewritten: only look a route pattern up.
        let original = (issue.target == PatternTarget::Route)
            .then(|| {
                rewritten
                    .iter()
                    .find_map(|(original, stored)| (stored == &issue.pattern).then_some(original))
            })
            .flatten();
        match original {
            Some(original) => message.push_str(&format!(
                "  - route pattern '{original}' (stored as '{}' by --rewrite-legacy-routes): {}\n",
                issue.pattern, issue.reason
            )),
            None => message.push_str(&format!(
                "  - {} pattern '{}': {}\n",
                issue.target.noun(),
                issue.pattern,
                issue.reason
            )),
        }
    }
    message.push_str(
        "Routes and redirects are stored as complete sets, so every pattern in \
         ws-resources.json must be valid before this deploy can proceed — including patterns \
         you did not change.",
    );
    message
}

/// The user text for one warning. `rewritten` (the `(original, stored)` pairs
/// from [`apply_route_rewrites`]) recovers the original spelling for
/// [`PatternIssueKind::LegacyAfterRewrite`]; the other kinds ignore it.
pub fn format_warning(issue: &PatternIssue, rewritten: &[(String, String)]) -> String {
    let PatternIssue {
        target,
        pattern,
        reason,
        kind,
    } = issue;
    match kind {
        PatternIssueKind::GlobInvalid => {
            let (noun, set) = (target.noun(), target.plural());
            format!(
                "On-chain {noun} pattern '{pattern}' is invalid ({reason}); portals skip it \
                 when matching. This deploy leaves {set} unchanged and can proceed, but fix \
                 the pattern in ws-resources.json: the next deploy that modifies {set} will \
                 refuse to store it."
            )
        }
        PatternIssueKind::LegacySkipped => format!(
            "Route pattern '{pattern}' is valid glob syntax, but exceeds the legacy matcher's \
             limits ({reason}); portals still running legacy (regex) routing skip it. It only \
             takes effect on portals with glob routing enabled."
        ),
        PatternIssueKind::LegacyAfterRewrite => {
            let original = rewritten
                .iter()
                .find_map(|(original, stored)| (stored == pattern).then_some(original.as_str()))
                .unwrap_or(pattern.as_str());
            format!(
                "'{original}' was rewritten to '{pattern}', which portals still running legacy \
                 (regex) routing cannot match ({reason}) and will skip. Only deploy with \
                 --rewrite-legacy-routes once the portal fleet runs glob routing."
            )
        }
        PatternIssueKind::StarNarrowing => format!(
            "Note on route pattern '{pattern}': under glob routing, '*' matches within a \
             single path segment (legacy regex routing let it cross '/'). The pattern is \
             valid and stored as-is; if you want it to match everything below a prefix, use \
             a whole-segment '**' — e.g. '/foo/**' matches '/foo' and everything under it."
        ),
    }
}

/// The `--rewrite-legacy-routes` notice: which route patterns are stored in
/// rewritten (glob) form.
pub fn format_rewrite_notice(rewritten: &[(String, String)]) -> String {
    let mut message = format!(
        "--rewrite-legacy-routes: storing {} route pattern(s) in glob form (ws-resources.json \
         is not modified):",
        rewritten.len()
    );
    for (original, stored) in rewritten {
        message.push_str(&format!("\n  - '{original}' -> '{stored}'"));
    }
    message
}

/// The blocking message for rewrite collisions, one line per contested key.
pub fn format_rewrite_collisions(collisions: &[RewriteCollision]) -> String {
    collisions
        .iter()
        .map(|collision| {
            let originals = collision
                .originals
                .iter()
                .map(|original| format!("'{original}'"))
                .collect::<Vec<_>>()
                .join(" and ");
            format!(
                "--rewrite-legacy-routes produced conflicting route patterns: {originals} both \
                 rewrite to '{}'. Remove or merge these entries in ws-resources.json.",
                collision.rewritten
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
