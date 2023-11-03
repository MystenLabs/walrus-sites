/// The module exposes the functionality to create and update blocksites.
module blocksite::blocksite {
    use std::option::{Option, none, some};
    use sui::clock::{Self, Clock};
    use sui::transfer;
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use std::vector;

    /// The blocksite
    struct BlockSite has key, store {
        id: UID,
        // The date and time of creation
        created: u64,
        // The date and time latest update
        updated: Option<u64>, 
        // The number of times this site has been updated
        version: u64,
        contents: vector<u8>,
    }

    public fun create(contents: vector<u8>, clk: &clock::Clock, ctx: &mut TxContext): BlockSite {
        BlockSite {
            id: object::new(ctx),
            created: clock::timestamp_ms(clk),
            updated: none(),
            version: 1,
            contents,
            }
    }

    /// For use with the command line
    public fun create_to_sender(contents: vector<u8>, clk: &Clock, ctx: &mut TxContext) {
        let site = create(contents, clk, ctx);
        transfer::transfer(site, tx_context::sender(ctx));
    }

    /// Update the contents of the BlockSite, and increment version number and updated timestamps
    public fun update(site: &mut BlockSite, contents: vector<u8>, clk: &clock::Clock) {
        site.contents = contents;
        site.updated = some(clock::timestamp_ms(clk));
        site.version = site.version + 1;
    }

    /// Add more bytes to the content
    /// Since the size of the blocksite may be bigger than the max argument size
    /// (about 16KB) we add pieces to the contents later on to complete the
    /// site.
    public fun add_piece(site: &mut BlockSite, piece: vector<u8>, clk: &Clock) {
        vector::append(&mut site.contents, piece);
        site.updated = some(clock::timestamp_ms(clk));
    }

    #[test]
    fun test_create() {
        use sui::test_scenario;

        let scenario_val = test_scenario::begin(@0x42);
        let scenario = &mut scenario_val;

        // Create the block
        let cur_clock = clock::create_for_testing(test_scenario::ctx(scenario));
        let site = create(b"<h1>Hello!</h1>", &cur_clock, test_scenario::ctx(scenario));
        // Transfer to the user
        transfer::public_transfer(site, @0xc0ffee);

        // Add piece
        test_scenario::next_tx(scenario, @0xc0ffee);
        let site = test_scenario::take_from_sender<BlockSite>(scenario);
        add_piece(&mut site, b"<h2>bye</h2>", &cur_clock);
        assert!(site.contents == b"<h1>Hello!</h1><h2>bye</h2>", 0);
        test_scenario::return_to_sender(scenario, site);

        clock::destroy_for_testing(cur_clock);
        test_scenario::end(scenario_val);
    }
}
