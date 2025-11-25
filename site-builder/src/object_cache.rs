// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use sui_types::base_types::{ObjectID, ObjectRef};

#[derive(Default)]
pub struct ObjectCache(HashMap<ObjectID, ObjectRef>);

impl ObjectCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: ObjectID, obj_ref: ObjectRef) -> Option<ObjectRef> {
        self.0.insert(id, obj_ref)
    }

    pub fn get(&self, id: &ObjectID) -> Option<&ObjectRef> {
        self.0.get(id)
    }
}
