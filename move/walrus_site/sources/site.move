/// The module exposes the functionality to create and update Walrus sites.
module walrus_site::site;

use std::string::String;
use sui::display::{Self, Display};
use sui::dynamic_field as df;
use sui::package::{Self, Publisher};
use sui::vec_map;
use suins::controller;
use suins::suins::SuiNS;
use walrus_site::events::{emit_site_created, emit_site_burned};
use walrus_site::metadata::Metadata;
use walrus_site::redirects::{Redirects, redirects_field};

use fun df::add as UID.df_add;
use fun df::borrow as UID.df;
use fun df::borrow_mut as UID.df_mut;
use fun df::exists_ as UID.df_exists;
use fun df::remove as UID.df_remove;
use fun df::remove_if_exists as UID.df_remove_if_exists;

/// The name of the dynamic field containing the routes.
const ROUTES_FIELD: vector<u8> = b"routes";

// Abort code no longer used
// const EResourceDoesNotExist: u64 = 0;
const ERangeStartGreaterThanRangeEnd: u64 = 1;
const EStartAndEndRangeAreNone: u64 = 2;
/// Redirects must be removed before burning a site to reclaim storage rebates.
const ERemoveRedirectsFirst: u64 = 3;
/// Routes must be removed before burning a site to reclaim storage rebates.
const ERemoveRoutesFirst: u64 = 4;

/// The site published on Sui.
public struct Site has key, store {
    id: UID,
    name: String,
    link: Option<String>,
    image_url: Option<String>,
    description: Option<String>,
    project_url: Option<String>,
    creator: Option<String>,
}

/// A resource in a site.
public struct Resource has drop, store {
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

public struct Range has drop, store {
    start: Option<u64>, // inclusive lower bound
    end: Option<u64>, // inclusive upper bound
}

/// Representation of the resource path.
///
/// Ensures there are no namespace collisions in the dynamic fields.
public struct ResourcePath has copy, drop, store {
    path: String,
}

/// The routes for a site.
public struct Routes has drop, store {
    route_list: vec_map::VecMap<String, String>,
}

/// One-Time-Witness for the module.
public struct SITE has drop {}

fun init(otw: SITE, ctx: &mut TxContext) {
    let publisher = package::claim(otw, ctx);
    let d = init_site_display(&publisher, ctx);
    transfer::public_transfer(d, ctx.sender());
    transfer::public_transfer(publisher, ctx.sender());
}

/// Creates a new site.
public fun new_site(name: String, metadata: Metadata, ctx: &mut TxContext): Site {
    let site = Site {
        id: object::new(ctx),
        name,
        link: metadata.link(),
        image_url: metadata.image_url(),
        description: metadata.description(),
        project_url: metadata.project_url(),
        creator: metadata.creator(),
    };
    emit_site_created(
        object::id(&site),
    );
    site
}

/// Optionally creates a new Range object.
public fun new_range_option(range_start: Option<u64>, range_end: Option<u64>): Option<Range> {
    if (range_start.is_none() && range_end.is_none()) {
        return option::none<Range>()
    };
    option::some(new_range(range_start, range_end))
}

/// Creates a new Range object.
///
/// aborts if both range_start and range_end are none.
public fun new_range(range_start: Option<u64>, range_end: Option<u64>): Range {
    let start_is_defined = range_start.is_some();
    let end_is_defined = range_end.is_some();

    // At least one of the range bounds should be defined.
    assert!(start_is_defined || end_is_defined, EStartAndEndRangeAreNone);

    // If both range bounds are defined, the upper bound should be greater than the lower.
    if (start_is_defined && end_is_defined) {
        let start = option::borrow(&range_start);
        let end = option::borrow(&range_end);
        assert!(*end > *start, ERangeStartGreaterThanRangeEnd);
    };

    Range {
        start: range_start,
        end: range_end,
    }
}

/// Creates a new resource.
public fun new_resource(
    path: String,
    blob_id: u256,
    blob_hash: u256,
    range: Option<Range>,
): Resource {
    Resource {
        path,
        headers: vec_map::empty(),
        blob_id,
        blob_hash,
        range,
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

/// Update the site metadata.
public fun update_metadata(site: &mut Site, metadata: Metadata) {
    site.link = metadata.link();
    site.image_url = metadata.image_url();
    site.description = metadata.description();
    site.project_url = metadata.project_url();
    site.creator = metadata.creator();
}

/// Adds a resource to an existing site.
public fun add_resource(site: &mut Site, resource: Resource) {
    let path_obj = new_path(resource.path);
    df::add(&mut site.id, path_obj, resource);
}

/// Removes a resource from a site.
///
/// Aborts if the resource does not exist.
public fun remove_resource(site: &mut Site, path: String): Resource {
    let path_obj = new_path(path);
    df::remove(&mut site.id, path_obj)
}

/// Removes a resource from a site if it exists.
public fun remove_resource_if_exists(site: &mut Site, path: String): Option<Resource> {
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

/// Populates the routes dynamic field with the given entries.
///
/// Entries are inserted in reverse order (via `zip_do_reverse!`), so the caller
/// should pass the vectors in reverse of the desired VecMap order, if ordering is important.
///
/// Aborts if any route source is duplicated.
public fun fill_routes(self: &mut Site, from: vector<String>, to: vector<String>) {
    let Routes { route_list } = self.id.df_mut(ROUTES_FIELD);
    from.zip_do_reverse!(to, |from, to| route_list.insert(from, to));
}

/// Remove all routes from the site.
public fun remove_all_routes_if_exist(site: &mut Site): Option<Routes> {
    df::remove_if_exists(&mut site.id, ROUTES_FIELD)
}

/// Add a route to the site.
///
/// The insertion operation fails if the route already exists.
public fun insert_route(site: &mut Site, route: String, resource_path: String) {
    let routes = df::borrow_mut(&mut site.id, ROUTES_FIELD);
    routes_insert(routes, route, resource_path);
}

/// Remove a route from the site.
public fun remove_route(site: &mut Site, route: &String): (String, String) {
    let routes = df::borrow_mut(&mut site.id, ROUTES_FIELD);
    routes_remove(routes, route)
}

// === redirects ===

/// Returns an immutable reference to the redirects dynamic field.
///
/// Aborts if the redirects dynamic field does not exist.
public fun redirects(self: &Site): &Redirects {
    self.id.df(redirects_field!())
}

/// Adds the redirects dynamic field to the site.
///
/// Aborts if the redirects dynamic field already exists.
public fun set_redirects(self: &mut Site, redirects: Redirects) {
    self.id.df_add(redirects_field!(), redirects);
}

/// Populates the redirects dynamic field with the given entries.
///
/// Entries are inserted in reverse order (via `pop_back`), so the caller should
/// pass the vectors in reverse of the desired VecMap order, if ordering is important.
///
/// Aborts if the vectors have different lengths, any status code is invalid,
/// or any path is duplicated.
public fun fill_redirects(
    self: &mut Site,
    from: vector<String>,
    to: vector<String>,
    status_codes: vector<u16>,
) {
    let redirects: &mut Redirects = self.id.df_mut(redirects_field!());
    redirects.fill(from, to, status_codes)
}

/// Removes and returns the redirects dynamic field from the site.
///
/// Aborts if the redirects dynamic field does not exist.
public fun take_redirects(self: &mut Site): Redirects {
    self.id.df_remove(redirects_field!())
}

/// Removes and returns the redirects dynamic field if it exists.
public fun take_redirects_if_exist(self: &mut Site): Option<Redirects> {
    self.id.df_remove_if_exists(redirects_field!())
}

/// Adds a single redirect to the site.
///
/// Aborts if the path already exists or the status code is invalid.
public fun insert_redirect(self: &mut Site, from: String, to: String, status_code: u16) {
    let redirects: &mut Redirects = self.id.df_mut(redirects_field!());
    redirects.insert(from, to, status_code)
}

/// Removes a single redirect from the site.
///
/// Aborts if the redirect does not exist.
public fun remove_redirect(self: &mut Site, from: &String): (String, String, u16) {
    let redirects: &mut Redirects = self.id.df_mut(redirects_field!());
    redirects.remove(from)
}

/// Deletes a site object.
///
/// Routes and redirects must be removed before calling this function, so
/// that their storage rebates are returned to the caller. Aborts with
/// `ERemoveRoutesFirst` or `ERemoveRedirectsFirst` if either dynamic field
/// still exists.
///
/// NB: Resource dynamic fields are **not** checked — the caller is responsible
/// for removing them beforehand. Any resources left attached become
/// inaccessible and their storage rebates are lost.
public fun burn(site: Site) {
    let Site {
        id,
        ..,
    } = site;
    assert!(!id.df_exists(ROUTES_FIELD), ERemoveRoutesFirst);
    assert!(!id.df_exists(redirects_field!()), ERemoveRedirectsFirst);
    emit_site_burned(id.to_inner());
    id.delete();
}

// ============================================= SuiNS =============================================

public fun set_suins_reverse_lookup(self: &mut Site, suins: &mut SuiNS, domain_name: String) {
    controller::set_object_reverse_lookup(suins, self.uid_mut(), domain_name);
}

/// Define a Display for the Site objects.
fun init_site_display(publisher: &Publisher, ctx: &mut TxContext): Display<Site> {
    let keys = vector[
        b"name".to_string(),
        b"link".to_string(),
        b"image_url".to_string(),
        b"description".to_string(),
        b"project_url".to_string(),
        b"creator".to_string(),
    ];

    let values = vector[
        b"{name}".to_string(),
        b"{link}".to_string(),
        b"{image_url}".to_string(),
        b"{description}".to_string(),
        b"{project_url}".to_string(),
        b"{creator}".to_string(),
    ];

    let mut d = display::new_with_fields<Site>(
        publisher,
        keys,
        values,
        ctx,
    );

    d.update_version();
    d
}

public fun get_site_name(site: &Site): String {
    site.name
}

public fun get_site_link(site: &Site): Option<String> {
    site.link
}

public fun get_site_image_url(site: &Site): Option<String> {
    site.image_url
}

public fun get_site_description(site: &Site): Option<String> {
    site.description
}

public fun get_site_project_url(site: &Site): Option<String> {
    site.project_url
}

public fun get_site_creator(site: &Site): Option<String> {
    site.creator
}

// ============================== public(package) ==============================

public(package) fun uid_mut(self: &mut Site): &mut UID {
    &mut self.id
}

#[test_only]
public fun init_for_testing(ctx: &mut TxContext) {
    init(SITE {}, ctx);
}
