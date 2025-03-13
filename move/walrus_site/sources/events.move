module walrus_site::events {
    use std::string::String;
    use sui::event;
    use walrus_site::metadata::Metadata;

    public struct SiteCreatedEvent has copy, drop {
        site_id: ID,
        site_name: String,
        site_metadata_link: Option<String>,
        site_metadata_image_url: Option<String>,
        site_metadata_description: Option<String>,
        site_metadata_project_url: Option<String>,
        site_metadata_creator: Option<String>,
    }

    public struct SiteBurnedEvent has copy, drop {
        site_id: ID,
    }

    public struct SiteNameUpdateEvent has copy, drop {
        site_id: ID,
        old_name: String,
        new_name: String,
    }

    public(package) fun emit_site_created(site_id: ID, name: String, metadata: &Metadata) {
        event::emit(SiteCreatedEvent {
            site_id,
            site_name: name,
            site_metadata_link: metadata.link(),
            site_metadata_image_url: metadata.image_url(),
            site_metadata_description: metadata.description(),
            site_metadata_project_url: metadata.project_url(),
            site_metadata_creator: metadata.creator(),
        });
    }

    public(package) fun emit_site_burned(site_id: ID) {
        event::emit(SiteBurnedEvent {
            site_id,
        });
    }

    public(package) fun emit_site_update_name(site_id: ID, old_name: String, new_name: String) {
        event::emit(SiteNameUpdateEvent {
            site_id,
            old_name,
            new_name,
        });
    }
}
