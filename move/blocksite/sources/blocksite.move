/// The module exposes the functionality to create and update blocksites.
module blocksite::blocksite {
    use std::option::{Option, none, some};
    use sui::clock::{Self, Clock};
    use sui::transfer;
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::dynamic_field as df;
    use std::string::String;

    /// The blocksite
    struct BlockSite has key, store {
        id: UID,
        name: String,
        // The date and time of creation
        created: u64,
    }

    // TODO: is the version/timestamp useful?
    struct BlockResource has store, drop {
        name: String,
        // The date and time of creation
        created: u64,
        // The date and time latest update
        updated: Option<u64>,
        // The number of times this site has been updated
        version: u64,
        content_type: String,
        content_encoding: String,
        // The walrus blob id containing the bytes for this resource
        blob_id: String
    }

    public fun new_site(name: String, clk: &Clock, ctx: &mut TxContext): BlockSite {
        BlockSite {
            id: object::new(ctx),
            name,
            created: clock::timestamp_ms(clk),
        }
    }

    public fun update_name(site: &mut BlockSite, new_name: String) {
        site.name = new_name
    }


    #[lint_allow(self_transfer)]
    /// For use with the command line
    public fun new_site_to_sender(name: String, clk: &Clock, ctx: &mut TxContext) {
        let site = new_site(name, clk, ctx);
        transfer::transfer(site, tx_context::sender(ctx));
    }

    // Manipulation of resources //

    public fun new_resource(
        name: String,
        content_type: String,
        content_encoding: String,
        blob_id: String,
        clk: &Clock,
    ): BlockResource {
        BlockResource {
            name,
            created: clock::timestamp_ms(clk),
            updated: none(),
            version: 1,
            content_type,
            content_encoding,
            blob_id,
        }
    }

    #[lint_allow(self_transfer)]
    /// Create a new site with a first resource already present.
    /// Testing function to speed up development.
    public fun new_site_with_resource_to_sender(
        site_name: String,
        resource_name: String,
        content_type: String,
        content_encoding: String,
        blob_id: String,
        clk: &Clock,
        ctx: &mut TxContext
    ) {
        let resource = new_resource(resource_name, content_type, content_encoding, blob_id, clk);
        let site = new_site(site_name, clk, ctx);
        add_resource(&mut site, resource);
        transfer::transfer(site, tx_context::sender(ctx));
    }

    public fun add_resource(site: &mut BlockSite, resource: BlockResource) {
        df::add(&mut site.id, resource.name, resource);
    }

    public fun remove_resource(site: &mut BlockSite, name: String): BlockResource{
        df::remove(&mut site.id, name)
    }

    public fun remove_resource_if_exists(site: &mut BlockSite, name: String): Option<BlockResource>{
        df::remove_if_exists(&mut site.id, name)
    }

    public fun move_resource(site: &mut BlockSite, old_name: String, new_name: String) {
        let resource = remove_resource(site, old_name);
        resource.name = new_name;
        add_resource(site, resource);
    }

    /// Update the contents of the resource, and increment version number and updated timestamps
    public fun update_blob_id(resource: &mut BlockResource, blob_id: String, clk: &Clock) {
        resource.blob_id = blob_id;
        resource.updated = some(clock::timestamp_ms(clk));
        resource.version = resource.version + 1;
    }

    public fun update_content_type(resource: &mut BlockResource, content_type: String, clk: &Clock) {
        resource.content_type = content_type;
        resource.updated = some(clock::timestamp_ms(clk));
        resource.version = resource.version + 1;
    }

    public fun update_content_encodng(resource: &mut BlockResource, content_encoding: String, clk: &Clock) {
        resource.content_encoding = content_encoding;
        resource.updated = some(clock::timestamp_ms(clk));
        resource.version = resource.version + 1;
    }
}
