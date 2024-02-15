/// The module exposes the functionality to create and update blocksites.
module blocksite::blocksite {
    use std::option::{Option, none, some};
    use sui::clock::{Self, Clock};
    use sui::transfer;
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::dynamic_field as df;
    use std::string::String;
    use std::vector;

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
        parts: u64,
        contents: vector<u8>,
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
        parts: u64,
        contents: vector<u8>,
        clk: &Clock,
    ): BlockResource {
        BlockResource {
            name,
            created: clock::timestamp_ms(clk),
            updated: none(),
            version: 1,
            content_type,
            content_encoding,
            parts,
            contents,
        }
    }

    public fun add_resource(node: &mut BlockSite, resource: BlockResource) {
        df::add(&mut node.id, resource.name, resource);
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
    public fun update_contents(resource: &mut BlockResource, contents: vector<u8>, clk: &Clock) {
        resource.contents = contents;
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

    /// Add more bytes to the content
    public fun add_piece(resource: &mut BlockResource, piece: vector<u8>, clk: &Clock) {
        vector::append(&mut resource.contents, piece);
        resource.updated = some(clock::timestamp_ms(clk));
    }

    public fun add_piece_to_existing(site: &mut BlockSite, name: String, piece: vector<u8>, clk: &Clock) {
        let resource = df::borrow_mut(&mut site.id, name);
        add_piece(resource, piece, clk);
    }
}
