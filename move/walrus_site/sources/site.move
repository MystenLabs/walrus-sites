/// The module exposes the functionality to create and update Walrus sites.
module walrus_site::site {
    use sui::dynamic_field as df;
    use std::string::String;
    use sui::vec_map;

    /// The name of the dynamic field containing the routes.
    const ROUTES_FIELD: vector<u8> = b"routes";

    /// An insertion of route was attempted, but the related resource does not exist.
    const EResourceDoesNotExist: u64 = 0;
    const ERangeStartGreaterThanRangeEnd: u64 = 1;
    const ERangeEndShouldNotBeZero: u64 = 2;

    /// The site published on Sui.
    public struct Site has key, store {
        id: UID,
        name: String,
    }

    /// A resource in a site.
    public struct Resource has store, drop {
        path: String,
        // Response, Representation and Payload headers
        // regarding the contents of the resource.
        headers: vec_map::VecMap<String, String>,
        // The walrus blob id containing the bytes for this resource.
        blob_id: u256,
        // Contains the hash of the contents of the blob
        // to verify its integrity.
        blob_hash: u256,
        // Defines the byte range of the resource contents
        // in the case where multiple resources are stored
        // in the same blob. This way, each resource will
        // be parsed using its' byte range in the blob.
        range: Option<Range>,
    }

    public struct Range has store, drop {
        start: Option<u256>, // inclusive lower bound
        end: Option<u256> // exclusive upper bound
    }

    /// Representation of the resource path.
    ///
    /// Ensures there are no namespace collisions in the dynamic fields.
    public struct ResourcePath has copy, store, drop {
        path: String,
    }

    /// The routes for a site.
    public struct Routes has store, drop {
        route_list: vec_map::VecMap<String, String>,
    }

    /// Creates a new site.
    public fun new_site(name: String, ctx: &mut TxContext): Site {
        Site {
            id: object::new(ctx),
            name,
        }
    }

    /// Used to create a Range object in order to pass it
    /// to the new_resource.
    public fun new_range(
        range_start: Option<u256>,
        range_end: Option<u256>
    ): Option<Range> {
        let start_is_defined = option::is_some(&range_start);
        let end_is_defined = option::is_some(&range_end);
        // If defined, upper range bound should be positive (zero excluded).
        if (end_is_defined) {
            let end = option::borrow(&range_end);
            assert!( *end != 0, ERangeEndShouldNotBeZero);
        };
        // If both range bounds are defined, the upper bound should be greater than the lower.
        if (start_is_defined && end_is_defined) {
            let start = option::borrow(&range_start);
            let end = option::borrow(&range_end);
            assert!(*end > *start, ERangeStartGreaterThanRangeEnd);
        };
        // Range is some, only if at least one of the range bounds is defined.
        if (!start_is_defined && !end_is_defined) {
            option::none()
        } else {
            option::some(
                Range {
                    start: range_start,
                    end: range_end
                }
            )
        }
    }

    /// Creates a new resource.
    public fun new_resource(
        path: String,
        blob_id: u256,
        blob_hash: u256,
        range: Option<Range>
    ): Resource {
        Resource {
            path,
            headers: vec_map::empty(),
            blob_id,
            blob_hash,
            range
        }
    }

    /// Adds a header to the Resource's headers vector.
    public fun add_header(resource: &mut Resource, name: String, value: String) {
        resource.headers.insert(name, value);
    }

    /// Creates a new resource path.
    fun new_path(path: String): ResourcePath {
        ResourcePath { path }
    }

    /// Updates the name of a site.
    public fun update_name(site: &mut Site, new_name: String) {
        site.name = new_name
    }

    /// Adds a resource to an existing site.
    public fun add_resource(site: &mut Site, resource: Resource) {
        let path_obj = new_path(resource.path);
        df::add(&mut site.id, path_obj, resource);
    }

    /// Removes a resource from a site.
    ///
    /// Aborts if the resource does not exist.
    public fun remove_resource(site: &mut Site, path: String): Resource{
        let path_obj = new_path(path);
        df::remove(&mut site.id, path_obj)
    }

    /// Removes a resource from a site if it exists.
    public fun remove_resource_if_exists(site: &mut Site, path: String): Option<Resource>{
        let path_obj = new_path(path);
        df::remove_if_exists(&mut site.id, path_obj)
    }

    /// Changes the path of a resource on a site.
    public fun move_resource(site: &mut Site, old_path: String, new_path: String) {
        let mut resource = remove_resource(site, old_path);
        resource.path = new_path;
        add_resource(site, resource);
    }

    // Routes.

    /// Creates a new `Routes` object.
    fun new_routes(): Routes {
        Routes { route_list: vec_map::empty() }
    }

    /// Inserts a route into the `Routes` object.
    ///
    /// The insertion operation fails if the route already exists.
    fun routes_insert(routes: &mut Routes, route: String, resource_path: String) {
        routes.route_list.insert(route, resource_path);
    }

    /// Removes a route from the `Routes` object.
    fun routes_remove(routes: &mut Routes, route: &String): (String, String) {
        routes.route_list.remove(route)
    }

    // Routes management on the site.

    /// Add the routes dynamic field to the site.
    public fun create_routes(site: &mut Site) {
        let routes = new_routes();
        df::add(&mut site.id, ROUTES_FIELD, routes);
    }

    /// Remove all routes from the site.
    public fun remove_all_routes_if_exist(site: &mut Site): Option<Routes> {
        df::remove_if_exists(&mut site.id, ROUTES_FIELD)
    }

    /// Add a route to the site.
    ///
    /// The insertion operation fails:
    /// - if the route already exists; or
    /// - if the related resource path does not already exist as a dynamic field on the site.
    public fun insert_route(site: &mut Site, route: String, resource_path: String) {
        let path_obj = new_path(resource_path);
        assert!(df::exists_(&site.id, path_obj), EResourceDoesNotExist);
        let routes = df::borrow_mut(&mut site.id, ROUTES_FIELD);
        routes_insert(routes, route, resource_path);
    }

    /// Remove a route from the site.
    public fun remove_route(site: &mut Site, route: &String): (String, String) {
        let routes = df::borrow_mut(&mut site.id, ROUTES_FIELD);
        routes_remove(routes, route)
    }
}
