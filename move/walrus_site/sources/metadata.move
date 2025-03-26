module walrus_site::metadata;

use std::string::String;

/// A struct that contains the Site's metadata.
public struct Metadata has copy, drop, store {
    link: Option<String>,
    image_url: Option<String>,
    description: Option<String>,
    project_url: Option<String>,
    creator: Option<String>,
}

/// Creates a new Metadata object.
public fun new_metadata(
    link: Option<String>,
    image_url: Option<String>,
    description: Option<String>,
    project_url: Option<String>,
    creator: Option<String>,
): Metadata {
    Metadata {
        link,
        image_url,
        description,
        project_url,
        creator,
    }
}

public fun link(metadata: &Metadata): Option<String> {
    metadata.link
}

public fun image_url(metadata: &Metadata): Option<String> {
    metadata.image_url
}

public fun description(metadata: &Metadata): Option<String> {
    metadata.description
}

public fun project_url(metadata: &Metadata): Option<String> {
    metadata.project_url
}

public fun creator(metadata: &Metadata): Option<String> {
    metadata.creator
}

public fun set_link(metadata: &mut Metadata, link: Option<String>) {
    metadata.link = link;
}

public fun set_image_url(metadata: &mut Metadata, image_url: Option<String>) {
    metadata.image_url = image_url;
}

public fun set_description(metadata: &mut Metadata, description: Option<String>) {
    metadata.description = description;
}

public fun set_project_url(metadata: &mut Metadata, project_url: Option<String>) {
    metadata.project_url = project_url;
}

public fun set_creator(metadata: &mut Metadata, creator: Option<String>) {
    metadata.creator = creator;
}
