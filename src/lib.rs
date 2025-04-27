mod protocol;

use std::ptr;
use std::os::raw::{c_char, c_int};
use std::ffi::CString;

use crate::protocol::{Protocol, Version};

#[repr(C)]
pub struct CProtocol {
    pub id: *const c_char,
    pub name: *const c_char,
    pub versions: *const CVersion,
    pub versions_len: usize,
}

#[repr(C)]
pub struct CVersion {
    pub version: c_int,
    pub query_commands: *const *const c_char,
    pub query_commands_len: usize,
}

impl CProtocol {
    pub fn from_protocol(protocol: &Protocol) -> Self {
        let id = CString::new(protocol.id.clone()).unwrap();
        let name = CString::new(protocol.name.clone()).unwrap();

        let versions: Vec<CVersion> = protocol
            .versions
            .iter()
            .map(|v| CVersion::from_version(v))
            .collect();

        let versions_ptr = versions.as_ptr();
        std::mem::forget(versions); // Éviter que Rust libère la mémoire

        CProtocol {
            id: id.into_raw(),
            name: name.into_raw(),
            versions: versions_ptr,
            versions_len: protocol.versions.len(),
        }
    }
}

impl CVersion {
    pub fn from_version(version: &Version) -> Self {
        let query_commands: Vec<*const c_char> = version
            .query_commands
            .iter()
            .map(|cmd| CString::new(cmd.clone()).unwrap().into_raw() as *const c_char)
            .collect();

        let query_commands_ptr = query_commands.as_ptr();
        std::mem::forget(query_commands);

        CVersion {
            version: version.version as c_int,
            query_commands: query_commands_ptr,
            query_commands_len: version.query_commands.len(),
        }
    }
}


#[no_mangle]
pub extern "C" fn protocol_new(data: *const u8, len: usize) -> *mut CProtocol {
    if data.is_null() || len == 0 {
        return std::ptr::null_mut();
    }

    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    let protocols = Protocol::new(slice);

    if let Some((_, protocol)) = protocols.into_iter().next() {
        let c_protocol = CProtocol::from_protocol(&protocol);
        Box::into_raw(Box::new(c_protocol))
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn protocol_free(ptr: *mut CProtocol) {
    if !ptr.is_null() {
        let _ = unsafe { Box::from_raw(ptr) }; // Libère la mémoire
    }
}

#[no_mangle]
pub extern "C" fn protocol_get_command(
    protocol: *const CProtocol,
    _version: c_int,
    command: *const c_char,
) -> *const c_char {
    if protocol.is_null() || command.is_null() {
        return ptr::null();
    }

    let _protocol = unsafe { &*protocol };
    let _rust_command = unsafe { CString::from_raw(command as *mut c_char) }
        .to_str()
        .unwrap_or_default()
        .to_string();

    let rust_protocol = Protocol {
        id: unsafe { CString::from_raw(_protocol.id as *mut c_char) }
            .to_str()
            .unwrap_or_default()
            .to_string(),
        name: unsafe { CString::from_raw(_protocol.name as *mut c_char) }
            .to_str()
            .unwrap_or_default()
            .to_string(),
        versions: unsafe {
            std::slice::from_raw_parts(_protocol.versions, _protocol.versions_len)
                .iter()
                .map(|v| Version {
                    version: v.version as i16,
                    query_commands: std::slice::from_raw_parts(
                        v.query_commands,
                        v.query_commands_len,
                    )
                    .iter()
                    .map(|&cmd| {
                        CString::from_raw(cmd as *mut c_char)
                            .to_str()
                            .unwrap_or_default()
                            .to_string()
                    })
                    .collect(),
                    commands: vec![], // Remplir si nécessaire
                })
                .collect()
        },
    };

    if let Some(command) = rust_protocol.get_command(_version as i16, &_rust_command) {
        return CString::new(command.cmd.clone()).unwrap().into_raw()
    } else {
        return ptr::null();
    }
}