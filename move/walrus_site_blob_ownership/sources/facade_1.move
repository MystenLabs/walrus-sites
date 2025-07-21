module walrus_site::facade_1;

use std::string::String;

use walrus::blob::Blob;

use walrus_site::resource::Resource;
use walrus_site::site::Site;

// ================= Blobs =================

public fun add_blob(self: &mut Site, blob: Blob) {
    self.site_data_mut().add_blob(blob)
}

public fun remove_blob(self: &mut Site, blob_id: u256): Blob {
    self.site_data_mut().remove_blob(blob_id)
}

// ================= Resources =================

public fun add_resource(self: &mut Site, resource: Resource) {
    self.site_data_mut().add_resource(resource)
}

public fun remove_resource(self: &mut Site, path: String): Resource {
    self.site_data_mut().remove_resource(path)
}

public fun remove_resource_if_exists(self: &mut Site, path: String): Option<Resource> {
    match (self.site_data().contains_resource(path)) {
        true => option::some(self.remove_resource(path)),
        false => option::none()
    }
}

public fun move_resource(self: &mut Site, old_path: String, new_path: String) {
    let mut resource = self.remove_resource(old_path);
    *resource.path_mut() = new_path;
    self.add_resource(resource);
}

// ================= Routes =================

public fun add_route(self: &mut Site, from: String, to: String) {
    self.site_data_mut().add_route(from, to);
}

public fun remove_route(self: &mut Site, from: String): String {
    self.site_data_mut().remove_route(from)
}
