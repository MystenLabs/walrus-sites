/// The module exposes the functionality to create and update blocksites.
module blocksite::blocksite {
    use std::option::Option;
    use sui::transfer;
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::dynamic_field as df;
    use std::string::String;

    /// The blocksite
    struct BlockSite has key, store {
        id: UID,
        name: String,
    }

    struct BlockResource has store, drop {
        path: String,
        content_type: String,
        content_encoding: String,
        // The walrus blob id containing the bytes for this resource
        blob_id: u256,
    }

    public fun new_site(name: String, ctx: &mut TxContext): BlockSite {
        BlockSite {
            id: object::new(ctx),
            name,
        }
    }

    public fun update_name(site: &mut BlockSite, new_name: String) {
        site.name = new_name
    }


    #[lint_allow(self_transfer)]
    /// For use with the command line
    public fun new_site_to_sender(name: String, ctx: &mut TxContext) {
        let site = new_site(name, ctx);
        transfer::transfer(site, tx_context::sender(ctx));
    }

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

    public fun add_resource(site: &mut BlockSite, resource: BlockResource) {
        df::add(&mut site.id, resource.path, resource);
    }

    public fun remove_resource(site: &mut BlockSite, path: String): BlockResource{
        df::remove(&mut site.id, path)
    }

    public fun remove_resource_if_exists(site: &mut BlockSite, path: String): Option<BlockResource>{
        df::remove_if_exists(&mut site.id, path)
    }

    public fun move_resource(site: &mut BlockSite, old_path: String, new_path: String) {
        let resource = remove_resource(site, old_path);
        resource.path = new_path;
        add_resource(site, resource);
    }
}
