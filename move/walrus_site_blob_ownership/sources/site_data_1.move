module walrus_site::site_data_1;

use std::string::String;

use sui::object_table::{Self, ObjectTable};
use sui::table::{Self, Table};
use sui::vec_set::VecSet;

use walrus::blob::Blob;

use walrus_site::resource::Resource;

// Adding resources steps:
// 1. First add blob
// 2. Then add resources
// 3. Add route if necessary
// Removing resources steps:
// 1. First remove resource
// 2. If blob is not referenced anywhere, also remove blob

public struct SiteData has store {
    resources: Table<String, Resource>,
    blobs: ObjectTable<u256, Blob>,
    // NOTE: Could there be a blob that is assigned to more than eg 1000 resources?
    blob_resources: Table<u256, VecSet<String>>,
    routes: Table<String, String>
}

public struct BlobBorrow {
    table_id: ID,
    blob_id: u256,
}

public fun new(ctx: &mut TxContext): SiteData {
    SiteData {
        resources: table::new(ctx),
        blobs: object_table::new(ctx),
        blob_resources: table::new(ctx),
        routes: table::new(ctx),
    }
}

public fun destroy_empty(self: SiteData) {
    let SiteData {
        resources,
        blobs,
        blob_resources,
        routes,
    } = self;
    resources.destroy_empty();
    blobs.destroy_empty();
    blob_resources.destroy_empty();
    routes.destroy_empty();
}

public fun drop(self: SiteData): ObjectTable<u256, Blob> {
    let SiteData {
        resources,
        blobs,
        blob_resources,
        routes,
    } = self;
    resources.drop();
    blob_resources.drop();
    routes.drop();
    blobs
}

// ================= Blobs =================

public fun add_blob(self: &mut SiteData, blob: Blob) {
    let blob_id = blob.blob_id();
    self.blobs.add(blob_id, blob);
}

public fun remove_blob(self: &mut SiteData, blob_id: u256): Blob {
    let blob_resources = self.blob_resources.remove(blob_id);
    assert!(blob_resources.is_empty());
    self.blobs.remove(blob_id)
}

public fun borrow_blob(self: &mut SiteData, blob_id: u256): (Blob, BlobBorrow) {
    (
        self.blobs.remove(blob_id),
        BlobBorrow {
            table_id: object::id(&self.blobs),
            blob_id,
        }
    )
}

public fun return_blob(self: &mut SiteData, blob: Blob, borrow: BlobBorrow) {
    let BlobBorrow {
        table_id,
        blob_id
    } = borrow;
    assert!(table_id == object::id(&self.blobs));
    assert!(blob.blob_id() == blob_id);
    self.blobs.add(blob_id, blob);
}

public fun contains_blob(self: &SiteData, blob_id: u256): bool {
    self.blobs.contains(blob_id)
}

// ================= Resources =================

// NOTE: We might want to add events here?
public fun add_resource(self: &mut SiteData, resource: Resource) {
    let blob_id = resource.blob_id();
    assert!(self.blobs.contains(blob_id));
    self.blob_resources[blob_id].insert(*resource.path());
    self.resources.add(*resource.path(), resource);
}

public fun remove_resource(self: &mut SiteData, path: String): Resource {
    let resource = self.resources.remove(path);
    let blob_id = resource.blob_id();
    let blob_resources = &mut self.blob_resources[blob_id];
    blob_resources.remove(resource.path());
    if (blob_resources.is_empty()) {
        self.blob_resources.remove(blob_id);
    };
    resource
}

public fun contains_resource(self: &SiteData, path: String): bool {
    self.resources.contains(path)
}

public fun resources(self: &SiteData): &Table<String, Resource> {
    &self.resources
}

// ================= Routes =================

public fun add_route(self: &mut SiteData, from: String, to: String) {
    self.routes.add(from, to);
}

public fun remove_route(self: &mut SiteData, from: String): String {
    self.routes.remove(from)
}
