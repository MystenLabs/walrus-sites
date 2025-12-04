// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Size-based file grouping for optimal quilt packing.
//!
//! Groups files into buckets based on size to prevent small files from being
//! penalized by large files' column allocation overhead in Walrus quilt encoding.
//!
//! When files of vastly different sizes are placed in the same quilt, the column
//! allocation is dominated by the largest file, wasting space for smaller files.
//! This module provides a [`group_by_size`] function that partitions files into
//! size buckets with progressively stricter ratios for larger files.

use bytesize::ByteSize;

/// Size bucket boundaries (upper limits) for grouping files.
///
/// Files are placed into buckets based on which boundary they fall under.
/// The ratios between consecutive boundaries get stricter for larger files
/// because absolute waste scales with file size.
///
/// | Bucket | Range       | Ratio | Typical Content    |
/// |--------|-------------|-------|--------------------|
/// | 0      | 0-16KB      | 16x   | icons, small CSS   |
/// | 1      | 16-128KB    | 8x    | medium assets      |
/// | 2      | 128-512KB   | 4x    | images             |
/// | 3      | 512KB-2MB   | 4x    | large images       |
/// | 4      | 2-8MB       | 4x    | small videos       |
/// | 5      | 8-32MB      | 4x    | videos, PDFs       |
/// | 6      | 32-128MB    | 4x    | large media        |
/// | 7      | >128MB      | ∞     | huge files         |
const BUCKET_BOUNDARIES: &[ByteSize] = &[
    ByteSize::kib(16),  // 0-16KB (tiny)
    ByteSize::kib(128), // 16-128KB (small, 8x ratio)
    ByteSize::kib(512), // 128-512KB (medium, 4x ratio)
    ByteSize::mib(2),   // 512KB-2MB (large, 4x ratio)
    ByteSize::mib(8),   // 2-8MB (4x ratio)
    ByteSize::mib(32),  // 8-32MB (4x ratio)
    ByteSize::mib(128), // 32-128MB (4x ratio)
];

/// Groups items by size for optimal quilt packing.
///
/// Items are partitioned into buckets based on [`BUCKET_BOUNDARIES`]. Items in
/// different buckets should be stored in separate quilts to avoid small files
/// being penalized by large files' column allocation overhead.
///
/// # Arguments
///
/// * `items` - Items to group
/// * `size_fn` - Closure that extracts the size from an item as [`ByteSize`]
///
/// # Returns
///
/// A `Vec` of groups, ordered from largest bucket to smallest bucket.
/// Items within each group maintain their original input order.
pub fn group_by_size<T, F>(items: Vec<T>, size_fn: F) -> Vec<Vec<T>>
where
    F: Fn(&T) -> ByteSize,
{
    if items.is_empty() {
        return vec![];
    }

    let num_buckets = BUCKET_BOUNDARIES.len() + 1;
    let mut buckets: Vec<Vec<T>> = (0..num_buckets).map(|_| Vec::new()).collect();

    for item in items {
        let size = size_fn(&item);
        let bucket_idx = bucket_index(size);
        buckets[bucket_idx].push(item);
    }

    // Return non-empty buckets, largest files first (reverse order of bucket indices).
    // This ensures that large files are processed first, which can be beneficial
    // when the quilt store has limited capacity.
    buckets
        .into_iter()
        .enumerate()
        .rev()
        .filter_map(|(idx, bucket)| {
            if bucket.is_empty() {
                None
            } else {
                tracing::debug!(
                    bucket_idx = idx,
                    file_count = bucket.len(),
                    "Using size bucket"
                );
                Some(bucket)
            }
        })
        .collect()
}

/// Determines the bucket index for a given file size.
///
/// Returns the index of the first boundary that the size falls under,
/// or the number of boundaries if the size exceeds all of them.
#[inline]
fn bucket_index(size: ByteSize) -> usize {
    BUCKET_BOUNDARIES
        .iter()
        .position(|&boundary| size < boundary)
        .unwrap_or(BUCKET_BOUNDARIES.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let items: Vec<u64> = vec![];
        let groups = group_by_size(items, |&x| ByteSize::b(x));
        assert!(groups.is_empty());
    }

    #[test]
    fn single_item_returns_single_group() {
        let items = vec![100u64];
        let groups = group_by_size(items, |&x| ByteSize::b(x));

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec![100]);
    }

    #[test]
    fn separates_files_into_different_buckets() {
        let items = vec![
            ("tiny", 100u64),         // < 16KB -> bucket 0
            ("small", 50_000u64),     // 16-128KB -> bucket 1
            ("large", 10_000_000u64), // 8-32MB -> bucket 5
            ("tiny2", 200u64),        // < 16KB -> bucket 0
        ];

        let groups = group_by_size(items, |&(_, size)| ByteSize::b(size));

        // Should have 3 groups: large files, small files, tiny files
        assert_eq!(groups.len(), 3);

        // Largest files come first
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[0][0].0, "large");

        // Small files next
        assert_eq!(groups[1].len(), 1);
        assert_eq!(groups[1][0].0, "small");

        // Tiny files last (grouped together), preserving original order
        assert_eq!(groups[2].len(), 2);
        assert_eq!(groups[2][0].0, "tiny");
        assert_eq!(groups[2][1].0, "tiny2");
    }

    #[test]
    fn preserves_original_order_within_bucket() {
        let items = vec![("a", 100u64), ("b", 500u64), ("c", 200u64)];

        let groups = group_by_size(items, |&(_, size)| ByteSize::b(size));

        // All tiny files, should be in one bucket, preserving original order
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0][0].0, "a");
        assert_eq!(groups[0][1].0, "b");
        assert_eq!(groups[0][2].0, "c");
    }

    #[test]
    fn bucket_boundary_edge_cases() {
        // Test that boundaries are exclusive (size < boundary goes to lower bucket)
        let kb = 1024u64;
        let items = vec![
            16 * kb - 1,  // Just under 16KB -> bucket 0
            16 * kb,      // Exactly 16KB -> bucket 1
            128 * kb - 1, // Just under 128KB -> bucket 1
            128 * kb,     // Exactly 128KB -> bucket 2
        ];

        let groups = group_by_size(items, |&x| ByteSize::b(x));

        // Should have 3 groups
        assert_eq!(groups.len(), 3);

        // bucket 2: 128KB
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[0][0], 128 * kb);

        // bucket 1: 16KB and 128KB-1
        assert_eq!(groups[1].len(), 2);

        // bucket 0: 16KB-1
        assert_eq!(groups[2].len(), 1);
        assert_eq!(groups[2][0], 16 * kb - 1);
    }

    #[test]
    fn handles_very_large_files() {
        let mb = 1024 * 1024u64;
        let items = vec![
            ("huge1", 200 * mb), // 200MB -> bucket 7 (> 128MB)
            ("huge2", 500 * mb), // 500MB -> bucket 7 (> 128MB)
            ("large", 50 * mb),  // 50MB -> bucket 6 (32-128MB)
        ];

        let groups = group_by_size(items, |&(_, size)| ByteSize::b(size));

        // Should have 2 groups: huge files together, large file separate
        assert_eq!(groups.len(), 2);

        // Huge files (bucket 7) come first, preserving original order
        assert_eq!(groups[0].len(), 2);
        assert_eq!(groups[0][0].0, "huge1"); // original order preserved
        assert_eq!(groups[0][1].0, "huge2");

        // Large file (bucket 6)
        assert_eq!(groups[1].len(), 1);
        assert_eq!(groups[1][0].0, "large");
    }

    #[test]
    fn handles_zero_size_files() {
        let items = vec![("empty", 0u64), ("tiny", 100u64)];

        let groups = group_by_size(items, |&(_, size)| ByteSize::b(size));

        // Both should be in bucket 0 (< 16KB)
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn preserves_all_items() {
        let items: Vec<u64> = (0..100).map(|i| i * 1000).collect();
        let original_len = items.len();

        let groups = group_by_size(items, |&x| ByteSize::b(x));

        let total_items: usize = groups.iter().map(|g| g.len()).sum();
        assert_eq!(total_items, original_len);
    }

    #[test]
    fn bucket_index_boundaries() {
        // Test bucket_index function directly
        assert_eq!(bucket_index(ByteSize::b(0)), 0);
        assert_eq!(bucket_index(ByteSize::kib(16) - ByteSize::b(1)), 0);
        assert_eq!(bucket_index(ByteSize::kib(16)), 1);
        assert_eq!(bucket_index(ByteSize::kib(128) - ByteSize::b(1)), 1);
        assert_eq!(bucket_index(ByteSize::kib(128)), 2);
        assert_eq!(bucket_index(ByteSize::kib(512)), 3);
        assert_eq!(bucket_index(ByteSize::mib(2)), 4);
        assert_eq!(bucket_index(ByteSize::mib(8)), 5);
        assert_eq!(bucket_index(ByteSize::mib(32)), 6);
        assert_eq!(bucket_index(ByteSize::mib(128)), 7);
        assert_eq!(bucket_index(ByteSize::gib(1)), 7); // All huge files in last bucket
    }

    #[test]
    fn works_with_complex_types() {
        // Verify the generic implementation works with complex types.
        // This mirrors the real ResourceData struct which has `unencoded_size: usize`.
        struct FileInfo {
            name: String,
            unencoded_size: usize,
            #[allow(dead_code)]
            metadata: Vec<u8>,
        }

        let items = vec![
            FileInfo {
                name: "a".into(),
                unencoded_size: 100, // tiny bucket
                metadata: vec![1, 2, 3],
            },
            FileInfo {
                name: "b".into(),
                unencoded_size: 1_000_000, // large bucket (512KB-2MB)
                metadata: vec![],
            },
        ];

        // Conversion from usize to ByteSize mirrors the real call site in resource.rs
        let groups = group_by_size(items, |f| ByteSize::b(f.unencoded_size as u64));

        // Two groups: large bucket first (higher bucket index), tiny bucket second
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0][0].name, "b"); // Large file in first group (larger bucket)
        assert_eq!(groups[1][0].name, "a"); // Tiny file in second group (smaller bucket)
    }
}
