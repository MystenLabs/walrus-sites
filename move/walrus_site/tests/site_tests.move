#[test_only]
module walrus_site::site_tests {
    use walrus_site::site::{
        ERangeStartGreaterThanRangeEnd,
        EStartAndEndRangeAreNone,
        Site, Range
    };

    #[test]
    #[expected_failure(abort_code = EStartAndEndRangeAreNone)]
    fun test_new_range_no_bounds_defined() {
        walrus_site::site::new_range(
            option::none(),
            option::none()
        );
    }
    #[test]
    fun test_new_range_both_bounds_defined() {
        walrus_site::site::new_range(
            option::some(0),
            option::some(1)
        );
    }
    #[test]
    fun test_new_range_only_upper_bound_defined() {
        walrus_site::site::new_range(
            option::none(),
            option::some(1024)
        );
    }
    #[test]
    fun test_new_range_only_lower_bound_defined() {
        walrus_site::site::new_range(
            option::some(1024),
            option::none()
        );
    }
    #[test]
    fun test_new_range_lower_bound_can_be_zero() {
        walrus_site::site::new_range(
            option::some(0),
            option::none()
        );
    }
    #[test]
    #[expected_failure(abort_code = ERangeStartGreaterThanRangeEnd)]
    fun test_new_range_upper_cannot_be_less_than_lower_bound() {
        walrus_site::site::new_range(
            option::some(2),
            option::some(1)
        );
    }

    #[test]
    fun test_site_creation_with_resources_and_routes() {
        use sui::test_scenario;
        let owner = @0xCAFE;
        let mut scenario = test_scenario::begin(owner);
        // Create a site.
        {
            let site = walrus_site::site::new_site(
                b"Example".to_string(), scenario.ctx()
            );
            transfer::public_transfer(site, owner)
        };

        // Add a resource to the site.
        scenario.next_tx(owner);
        {
            let mut site = scenario.take_from_sender<Site>();
            let resource = walrus_site::site::new_resource(
               b"index.html".to_string(),
               601749199,
               124794210,
               option::none<Range>()
            );
            walrus_site::site::add_resource(&mut site, resource);
            scenario.return_to_sender<Site>(site);
        };

        scenario.end();
    }


}
