module walrus_site::site_blobs;

use sui::object_table::{Self, ObjectTable};

use walrus::blob::Blob;

// TODO: Decide on ObjectTable vs TTO
public struct SiteBlobs has store {
    blobs: ObjectTable<u256, Blob>,
    dirty: bool,
}

public struct BlobBorrow {
    table_id: ID,
    blob_id: u256,
}

public fun borrow(self: &mut SiteBlobs, blob_id: u256): (Blob, BlobBorrow) {
    (
        self.blobs.remove(blob_id),
        BlobBorrow {
            table_id: object::id(&self.blobs),
            blob_id,
        }
    )
}

public fun return_blob(self: &mut SiteBlobs, blob: Blob, borrow: BlobBorrow) {
    let BlobBorrow {
        table_id,
        blob_id
    } = borrow;
    assert!(table_id == object::id(&self.blobs));
    assert!(blob.blob_id() == blob_id);
    self.blobs.add(blob_id, blob);
}

public(package) fun new(ctx: &mut TxContext): SiteBlobs {
    SiteBlobs {
        blobs: object_table::new(ctx),
        dirty: false,
    }
}

public(package) fun destroy_empty(self: SiteBlobs) {
    let SiteBlobs {
        blobs,
        ..
    } = self;
    blobs.destroy_empty();
}

