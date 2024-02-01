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

    struct BlockPage has store {
        name: String,
        // The date and time of creation
        created: u64,
        // The date and time latest update
        updated: Option<u64>, 
        // The number of times this site has been updated
        version: u64,
        content_type: String,
        content_encoding: String,
        contents: vector<u8>,
    }

    public fun new_site(name: String, clk: &Clock, ctx: &mut TxContext): BlockSite {
        BlockSite {
            id: object::new(ctx),
            name,
            created: clock::timestamp_ms(clk),
        }
    }

    #[lint_allow(self_transfer)]
    /// For use with the command line
    public fun new_site_to_sender(name: String, clk: &Clock, ctx: &mut TxContext) {
        let site = new_site(name, clk, ctx);
        transfer::transfer(site, tx_context::sender(ctx));
    }

    // Manipulation of pages //

    public fun new_page(
        name: String,
        content_type: String,
        content_encoding: String,
        contents: vector<u8>,
        clk: &Clock,
    ): BlockPage {
        BlockPage {
            name,
            created: clock::timestamp_ms(clk),
            updated: none(),
            version: 1,
            content_type,
            content_encoding,
            contents,
        }
    }

    public fun add_page(node: &mut BlockSite, page: BlockPage) {
        df::add(&mut node.id, page.name, page);
    }

    public fun remove_page(site: &mut BlockSite, name: String): BlockPage{
        df::remove(&mut site.id, name)
    }

    // TODO: Update content encoding and content type too
    /// Update the contents of the page, and increment version number and updated timestamps
    public fun update(page: &mut BlockPage, contents: vector<u8>, clk: &Clock) {
        page.contents = contents;
        page.updated = some(clock::timestamp_ms(clk));
        page.version = page.version + 1;
    }

    /// Add more bytes to the content
    public fun add_piece(page: &mut BlockPage, piece: vector<u8>, clk: &Clock) {
        vector::append(&mut page.contents, piece);
        page.updated = some(clock::timestamp_ms(clk));
    }
}
