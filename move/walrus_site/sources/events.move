module walrus_site::events;

use sui::event;

public struct SiteCreatedEvent has copy, drop {
    site_id: ID,
}

public struct SiteBurnedEvent has copy, drop {
    site_id: ID,
}

public(package) fun emit_site_created(site_id: ID) {
    event::emit(SiteCreatedEvent {
        site_id,
    });
}

public(package) fun emit_site_burned(site_id: ID) {
    event::emit(SiteBurnedEvent {
        site_id,
    });
}
