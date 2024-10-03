/// The module exposes the functionality to create and update Walrus sites.
module walrus_site::site {
    use std::option::Option;
    use sui::object::{Self, UID};
    use sui::tx_context::TxContext;
    use sui::dynamic_field as df;
    use std::string::String;
    use sui::vec_map;

    /// The site published on Sui.
    struct Site has key, store {
        id: UID,
        name: String,
    }

    /// A resource in a site.
    struct Resource has store, drop {
        path: String,
        // Response, Representation and Payload headers
        // regarding the contents of the resource.
        headers: vec_map::VecMap<String, String>,
        // The walrus blob id containing the bytes for this resource.
        blob_id: u256,
        // Contains the hash of the contents of the blob
        // to verify its integrity.
        blob_hash: u256
    }

    /// Representation of the resource path.
    ///
    /// Ensures there are no namespace collisions in the dynamic fields.
    struct ResourcePath has copy, store, drop {
        path: String,
    }

    /// Creates a new site.
    public fun new_site(name: String, ctx: &mut TxContext): Site {
        Site {
            id: object::new(ctx),
            name,
        }
    }

    /// Creates a new resource.
    public fun new_resource(
        path: String,
        blob_id: u256,
        blob_hash: u256
    ): Resource {
        Resource {
            path,
            headers: vec_map::empty(),
            blob_id,
            blob_hash,
        }
    }

    /// Adds a header to the Resource's headers vector
    public fun add_header(resource: &mut Resource, name: String, value: String) {
        // Will throw an exception if duplicate key.
        vec_map::insert(
            &mut resource.headers,
            name,
            value
        );
    }

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
        let resource = remove_resource(site, old_path);
        resource.path = new_path;
        add_resource(site, resource);
    }
}
