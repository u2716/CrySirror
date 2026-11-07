// SPDX-License-Identifier: MPL-2.0
// Existing extracted-capability fixtures select their retained VM explicitly.
pub fn load(path:&std::path::Path,writable:&std::path::Path)->anyhow::Result<crysirror_runtime::Runtime> {
    crysirror_runtime::Runtime::with_registry(crysirror_runtime::model::Project::load(path)?,path,writable,crysirror_runtime::AdapterRegistry::with_reference_siglus())
}
