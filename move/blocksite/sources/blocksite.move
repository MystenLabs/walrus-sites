/// The module exposes the functionality to create and update blocksites.
module blocksite::blocksite {
    use std::option::Option;
    use sui::transfer;
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::dynamic_field as df;
    use std::string::String;

    /// The site published on Sui.
    struct BlockSite has key, store {
        id: UID,
        name: String,
    }

    /// A resource in a site.
    struct BlockResource has store, drop {
        path: String,
        content_type: String,
        content_encoding: String,
        // The walrus blob id containing the bytes for this resource
        blob_id: u256,
    }

    /// Creates a new site.
    public fun new_site(name: String, ctx: &mut TxContext): BlockSite {
        BlockSite {
            id: object::new(ctx),
            name,
        }
    }

    /// Updates the name of a site.
    public fun update_name(site: &mut BlockSite, new_name: String) {
        site.name = new_name
    }

    /// Creates a new resource.
    public fun new_resource(
        path: String,
        content_type: String,
        content_encoding: String,
        blob_id: u256,
    ): BlockResource {
        BlockResource {
            path,
            content_type,
            content_encoding,
            blob_id,
        }
    }

    /// Adds a resource to an existing site.
    public fun add_resource(site: &mut BlockSite, resource: BlockResource) {
        df::add(&mut site.id, resource.path, resource);
    }

    /// Removes a resource from a site.
    ///
    /// Aborts if the resource does not exist.
    public fun remove_resource(site: &mut BlockSite, path: String): BlockResource{
        df::remove(&mut site.id, path)
    }

    /// Removes a resource from a site if it exists.
    public fun remove_resource_if_exists(site: &mut BlockSite, path: String): Option<BlockResource>{
        df::remove_if_exists(&mut site.id, path)
    }

    /// Changes the path of a resource on a site.
    public fun move_resource(site: &mut BlockSite, old_path: String, new_path: String) {
        let resource = remove_resource(site, old_path);
        resource.path = new_path;
        add_resource(site, resource);
    }
}
