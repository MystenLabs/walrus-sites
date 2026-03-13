#[test_only]
module walrus_site::site_tests;

use sui::test_scenario;
use walrus_site::site::{
    ERangeStartGreaterThanRangeEnd,
    EStartAndEndRangeAreNone,
    Site,
    Range,
    init_for_testing,
    get_site_name,
    get_site_link,
    get_site_image_url,
    get_site_description,
    get_site_project_url,
    get_site_creator
};

#[test]
#[expected_failure(abort_code = EStartAndEndRangeAreNone)]
fun test_new_range_no_bounds_defined() {
    walrus_site::site::new_range(
        option::none(),
        option::none(),
    );
}
#[test]
fun test_new_range_both_bounds_defined() {
    walrus_site::site::new_range(
        option::some(0),
        option::some(1),
    );
}
#[test]
fun test_new_range_only_upper_bound_defined() {
    walrus_site::site::new_range(
        option::none(),
        option::some(1024),
    );
}
#[test]
fun test_new_range_only_lower_bound_defined() {
    walrus_site::site::new_range(
        option::some(1024),
        option::none(),
    );
}
#[test]
fun test_new_range_lower_bound_can_be_zero() {
    walrus_site::site::new_range(
        option::some(0),
        option::none(),
    );
}
#[test]
#[expected_failure(abort_code = ERangeStartGreaterThanRangeEnd)]
fun test_new_range_upper_cannot_be_less_than_lower_bound() {
    walrus_site::site::new_range(
        option::some(2),
        option::some(1),
    );
}

/// This test runs a typical process
/// checking many of the contract's functions.
#[test]
fun test_site_flow_with_resources_and_routes() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    // Create a site.
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::some(b"https://<b36>.wal.app".to_string()),
            option::some(b"https://<b36>.wal.app/image.png".to_string()),
            option::some(b"This is a test site.".to_string()),
            option::none(),
            option::none(),
        );
        let site = walrus_site::site::new_site(
            b"Example".to_string(),
            metadata,
            scenario.ctx(),
        );

        assert!(get_site_name(&site) == b"Example".to_string());
        assert!(get_site_link(&site).borrow() == b"https://<b36>.wal.app".to_string());
        assert!(
            get_site_image_url(&site).borrow() == b"https://<b36>.wal.app/image.png".to_string(),
        );
        assert!(get_site_description(&site).borrow() == b"This is a test site.".to_string());
        assert!(get_site_project_url(&site).is_none());
        assert!(get_site_creator(&site).is_none());

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
            option::none<Range>(),
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
        walrus_site::site::remove_resource(
            &mut site,
            b"styles.css".to_string(),
        );
        scenario.return_to_sender<Site>(site);
    };

    // Create a route, add it to the site.
    scenario.next_tx(owner);
    {
        let mut site = scenario.take_from_sender<Site>();
        // Create the routes DF.
        walrus_site::site::create_routes(&mut site);

        // Create a resource and add it to the site.
        // This is needed for the insert_route to work,
        // since objects from previous transactions are lost.
        let resource = walrus_site::site::new_resource(
            b"index.html".to_string(),
            601749199,
            124794210,
            option::none<Range>(),
        );
        walrus_site::site::add_resource(&mut site, resource);

        // Add some rerouting pairs.
        walrus_site::site::insert_route(
            &mut site,
            b"/path1".to_string(),
            b"index.html".to_string(),
        );
        walrus_site::site::insert_route(
            &mut site,
            b"/path2".to_string(),
            b"index.html".to_string(),
        );
        // Delete the last route.
        walrus_site::site::remove_route(
            &mut site,
            &b"/path2".to_string(),
        );

        // Remove all routes.
        walrus_site::site::remove_all_routes_if_exist(&mut site);
        scenario.return_to_sender<Site>(site);
    };

    // Burn the site.
    scenario.next_tx(owner);
    {
        let site = scenario.take_from_sender<Site>();
        walrus_site::site::burn(site);
    };
    scenario.end();
}

#[test]
fun test_update_metadata() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    // Create a site.
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::some(b"https://<b36>.wal.app".to_string()),
            option::some(b"https://<b36>.wal.app/image.png".to_string()),
            option::some(b"This is a test site.".to_string()),
            option::none(),
            option::none(),
        );
        let site = walrus_site::site::new_site(
            b"Example".to_string(),
            metadata,
            scenario.ctx(),
        );

        assert!(get_site_name(&site) == b"Example".to_string());
        assert!(get_site_link(&site).borrow() == b"https://<b36>.wal.app".to_string());

        transfer::public_transfer(site, owner)
    };

    // Rename site and add a resource with headers to the site.
    scenario.next_tx(owner);
    {
        let mut site = scenario.take_from_sender<Site>();
        // Update the site metadata.
        let metadata = walrus_site::metadata::new_metadata(
            option::some(b"https://<b36>.wal.app".to_string()),
            option::some(b"https://<b36>.wal.app/image.png".to_string()),
            option::some(b"I am just updating the site metadata.".to_string()),
            option::none(),
            option::none(),
        );
        site.update_metadata(metadata);
        transfer::public_transfer(site, owner)
    };
    scenario.end();
}

#[test]
fun test_init() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    {
        init_for_testing(scenario.ctx());
    };
    scenario.end();
}

// === fill_routes ===

#[test]
fun test_site_fill_routes() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::none(), option::none(), option::none(),
            option::none(), option::none(),
        );
        let mut site = walrus_site::site::new_site(
            b"Test".to_string(), metadata, scenario.ctx(),
        );

        walrus_site::site::create_routes(&mut site);
        site.fill_routes(
            vector[b"/path1".to_string(), b"/path2".to_string()],
            vector[b"index.html".to_string(), b"about.html".to_string()],
        );

        // Verify by removing the routes and checking they exist.
        walrus_site::site::remove_route(&mut site, &b"/path1".to_string());
        walrus_site::site::remove_route(&mut site, &b"/path2".to_string());

        walrus_site::site::remove_all_routes_if_exist(&mut site);
        walrus_site::site::burn(site);
    };
    scenario.end();
}

// === redirect tests on Site ===

#[test]
fun test_site_set_and_take_redirects() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::none(), option::none(), option::none(),
            option::none(), option::none(),
        );
        let mut site = walrus_site::site::new_site(
            b"Test".to_string(), metadata, scenario.ctx(),
        );

        let redirects = walrus_site::redirects::filled(
            vector[b"/old".to_string()],
            vector[b"/new".to_string()],
            vector[301],
        );
        site.set_redirects(redirects);

        let taken = site.take_redirects();
        assert!(taken.length() == 1);
        let (location, status_code) = taken.get(&b"/old".to_string());
        assert!(*location == b"/new".to_string());
        assert!(status_code == 301);

        walrus_site::site::burn(site);
    };
    scenario.end();
}

#[test]
fun test_site_insert_and_remove_redirect() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::none(), option::none(), option::none(),
            option::none(), option::none(),
        );
        let mut site = walrus_site::site::new_site(
            b"Test".to_string(), metadata, scenario.ctx(),
        );

        let redirects = walrus_site::redirects::empty();
        site.set_redirects(redirects);

        site.insert_redirect(b"/a".to_string(), b"/b".to_string(), 302);
        site.insert_redirect(b"/c".to_string(), b"https://example.com".to_string(), 308);

        let (path, location, status_code) = site.remove_redirect(&b"/a".to_string());
        assert!(path == b"/a".to_string());
        assert!(location == b"/b".to_string());
        assert!(status_code == 302);

        // Clean up the DF before burning.
        site.take_redirects();
        walrus_site::site::burn(site);
    };
    scenario.end();
}

#[test]
fun test_site_fill_redirects() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::none(), option::none(), option::none(),
            option::none(), option::none(),
        );
        let mut site = walrus_site::site::new_site(
            b"Test".to_string(), metadata, scenario.ctx(),
        );

        let redirects = walrus_site::redirects::empty();
        site.set_redirects(redirects);

        site.fill_redirects(
            vector[b"/a".to_string(), b"/b".to_string()],
            vector[b"/x".to_string(), b"/y".to_string()],
            vector[301, 307],
        );

        let taken = site.take_redirects();
        assert!(taken.length() == 2);
        let (location, status_code) = taken.get(&b"/a".to_string());
        assert!(*location == b"/x".to_string());
        assert!(status_code == 301);
        let (location, status_code) = taken.get(&b"/b".to_string());
        assert!(*location == b"/y".to_string());
        assert!(status_code == 307);

        walrus_site::site::burn(site);
    };
    scenario.end();
}

#[test]
fun test_site_take_redirects_if_exist() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::none(), option::none(), option::none(),
            option::none(), option::none(),
        );
        let mut site = walrus_site::site::new_site(
            b"Test".to_string(), metadata, scenario.ctx(),
        );

        // No redirects set yet — should return none.
        let result = site.take_redirects_if_exist();
        assert!(result.is_none());

        // Set redirects, then take — should return some.
        let redirects = walrus_site::redirects::filled(
            vector[b"/old".to_string()],
            vector[b"/new".to_string()],
            vector[301],
        );
        site.set_redirects(redirects);
        let result = site.take_redirects_if_exist();
        assert!(result.is_some());

        walrus_site::site::burn(site);
    };
    scenario.end();
}

#[test]
#[expected_failure(abort_code = walrus_site::redirects::EInvalidRedirectStatusCode)]
fun test_site_fill_redirects_invalid_status_code() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::none(), option::none(), option::none(),
            option::none(), option::none(),
        );
        let mut site = walrus_site::site::new_site(
            b"Test".to_string(), metadata, scenario.ctx(),
        );

        let redirects = walrus_site::redirects::empty();
        site.set_redirects(redirects);

        site.fill_redirects(
            vector[b"/a".to_string()],
            vector[b"/b".to_string()],
            vector[404],
        );

        site.take_redirects();
        walrus_site::site::burn(site);
    };
    scenario.end();
}

#[test]
#[expected_failure(abort_code = walrus_site::redirects::EInvalidRedirectStatusCode)]
fun test_site_insert_redirect_invalid_status_code() {
    let owner = @0xCAFE;
    let mut scenario = test_scenario::begin(owner);
    {
        let metadata = walrus_site::metadata::new_metadata(
            option::none(), option::none(), option::none(),
            option::none(), option::none(),
        );
        let mut site = walrus_site::site::new_site(
            b"Test".to_string(), metadata, scenario.ctx(),
        );

        let redirects = walrus_site::redirects::empty();
        site.set_redirects(redirects);

        site.insert_redirect(b"/a".to_string(), b"/b".to_string(), 500);

        site.take_redirects();
        walrus_site::site::burn(site);
    };
    scenario.end();
}
