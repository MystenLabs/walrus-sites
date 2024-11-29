#[test_only]
module walrus_site::site_tests {
    use walrus_site::site::{
        ERangeStartGreaterThanRangeEnd,
        EStartAndEndRangeAreNone,
        Site, Range, Resource
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

    /// This test runs a typical process
    /// checking many of the contract's functions.
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

        // Rename site and add a resource with headers to the site.
        scenario.next_tx(owner);
        {
            let mut site = scenario.take_from_sender<Site>();
            // Update the site name.
            walrus_site::site::update_name(&mut site, b"Fancy Example".to_string());
            // Create a resource.
            let mut resource = walrus_site::site::new_resource(
               b"index.html".to_string(),
               601749199,
               124794210,
               option::none<Range>()
            );
            // Add a header to the resource.
            walrus_site::site::add_header(
                &mut resource,
                b"Content-Type".to_string(),
                b"text/html; charset=utf-8".to_string(),
            );
            // Add the resource to the site.
            walrus_site::site::add_resource(&mut site, resource);
            // Move the resource to a different path.
            walrus_site::site::move_resource(
                &mut site,
                b"index.html".to_string(),
                b"styles.css".to_string(),
            );
            // Delete the resource.
            scenario.return_to_sender<Site>(site);
        };

        // TODO: create a route, add it to the site.

        // TODO: Delete the resources, delete the route, and finally delete the site.

        scenario.end();
    }


}
