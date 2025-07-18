/// The module exposes the functionality to create and update Walrus sites.
module walrus_site::site;

use std::string::String;
use sui::display::{Self, Display};
use sui::object_table::ObjectTable;
use sui::package::{Self, Publisher};
use sui::versioned::{Self, Versioned};

use walrus::blob::Blob;

use walrus_site::events_::{emit_site_created, emit_site_burned};
use walrus_site::metadata_::Metadata;
use walrus_site::site_data_1::{Self as site_data, SiteData};

const VERSION: u64 = 1;

/// The site published on Sui.
public struct Site has key, store {
    id: UID,
    site_data: Versioned,
    // to here
    name: String,
    link: Option<String>,
    image_url: Option<String>,
    description: Option<String>,
    project_url: Option<String>,
    creator: Option<String>,
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
        site_data: versioned::create(VERSION, site_data::new(ctx), ctx),
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

/// Deletes a site object.
///
/// NB: This function does **NOT** delete the dynamic fields! Make sure to call this function
/// after deleting manually all the dynamic fields attached to the sites object. If you don't
/// delete the dynamic fields, they will become unaccessible and you will not be able to delete
/// them in the future.
public fun burn(site: Site): ObjectTable<u256, Blob> {
    emit_site_burned(object::id(&site));
    let Site {
        id,
        site_data,
        ..
    } = site;
    id.delete();
    site_data.destroy<SiteData>().drop()
}

public fun destroy_empty(self: Site) {
    let Site {
        id,
        site_data,
        ..
    } = self;
    let site_id = id.to_inner();
    id.delete();
    emit_site_burned(site_id);
    site_data.destroy<SiteData>().destroy_empty()
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

#[test_only]
public fun init_for_testing(ctx: &mut TxContext) {
    init(SITE {}, ctx);
}

