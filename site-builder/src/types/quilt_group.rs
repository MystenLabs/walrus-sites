// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use bytesize::ByteSize;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct RerootedFilenameAndSize<'a> {
    /// filename is rerooted to the root of the site.
    pub filename: &'a str,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct QuiltGroup {
    pub patterns: Vec<glob::Pattern>,
    pub max_size: ByteSize,
}

impl QuiltGroup {
    #[allow(dead_code)] // TODO: Remove once we start integrating it
    pub fn has_match(&self, file: &RerootedFilenameAndSize) -> bool {
        if file.size > self.max_size.as_u64() {
            return false;
        }
        self.patterns.iter().any(|p| p.matches(file.filename))
    }
}

impl<'de> Deserialize<'de> for QuiltGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            patterns: Vec<String>,
            #[serde(alias = "maxSize")]
            max_size: ByteSize,
        }

        let Helper { patterns, max_size } = Helper::deserialize(deserializer)?;
        let patterns = patterns
            .into_iter()
            .map(|p| glob::Pattern::new(p.as_str()))
            .collect::<Result<Vec<_>, glob::PatternError>>()
            .map_err(|e| serde::de::Error::custom(format!("Failed parsing glob pattern: {e}")))?;

        Ok(QuiltGroup { patterns, max_size })
    }
}

impl Serialize for QuiltGroup {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("QuiltGroup", 2)?;
        let str_vec = self
            .patterns
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<&'_ str>>();
        s.serialize_field("patterns", &str_vec)?;
        s.serialize_field("maxSize", &self.max_size)?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use glob::Pattern;

    use super::*;

    #[test]
    fn test_match_within_size() {
        let g = QuiltGroup {
            patterns: vec![
                Pattern::new("*.css").unwrap(),
                Pattern::new("images/*").unwrap(),
            ],
            max_size: ByteSize::from_str("10KB").unwrap(),
        };
        let f = RerootedFilenameAndSize {
            filename: "/assets/app.css",
            size: 512,
        };
        assert!(g.has_match(&f));
    }

    #[test]
    fn test_over_size_no_match() {
        let g = QuiltGroup {
            patterns: vec![Pattern::new("*.css").unwrap()],
            max_size: ByteSize::from_str("1KB").unwrap(),
        };
        let f = RerootedFilenameAndSize {
            filename: "/assets/app.css",
            size: 2 * 1024,
        };
        assert!(!g.has_match(&f));
    }

    #[test]
    fn test_no_pattern_no_match() {
        let g = QuiltGroup {
            patterns: vec![
                Pattern::new("images/*").unwrap(),
                Pattern::new("*.json").unwrap(),
            ],
            max_size: ByteSize::from_str("10KB").unwrap(),
        };
        let f = RerootedFilenameAndSize {
            filename: "/assets/app.css",
            size: 512,
        };
        assert!(!g.has_match(&f));
    }

    #[test]
    fn test_boundary_equal_max_kb() {
        let g = QuiltGroup {
            patterns: vec![Pattern::new("*").unwrap()],
            max_size: ByteSize::from_str("1KB").unwrap(),
        };
        let f = RerootedFilenameAndSize {
            filename: "/anything",
            size: 1000,
        };
        assert!(g.has_match(&f));
    }

    #[test]
    fn test_boundary_equal_max_kib() {
        let g = QuiltGroup {
            patterns: vec![Pattern::new("*").unwrap()],
            max_size: ByteSize::from_str("1KiB").unwrap(),
        };
        let f = RerootedFilenameAndSize {
            filename: "/anything",
            size: 1024,
        };
        assert!(g.has_match(&f));
    }

    #[test]
    fn test_multiple_patterns() {
        let g = QuiltGroup {
            patterns: vec![
                Pattern::new("*.html").unwrap(),
                Pattern::new("*.css").unwrap(),
                Pattern::new("images/*").unwrap(),
            ],
            max_size: ByteSize::from_str("10KB").unwrap(),
        };
        let f1 = RerootedFilenameAndSize {
            filename: "/index.html",
            size: 100,
        };
        let f2 = RerootedFilenameAndSize {
            filename: "/styles/site.css",
            size: 100,
        };
        let f3 = RerootedFilenameAndSize {
            filename: "images/logo.png",
            size: 100,
        };
        let f4 = RerootedFilenameAndSize {
            filename: "/scripts/app.js",
            size: 100,
        };
        assert!(g.has_match(&f1));
        assert!(g.has_match(&f2));
        assert!(g.has_match(&f3));
        assert!(!g.has_match(&f4));
    }
}
