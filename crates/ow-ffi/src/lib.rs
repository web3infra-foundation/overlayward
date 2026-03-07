use std::ffi::{c_char, CStr, CString};
use std::ptr;

fn rt() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}

unsafe fn ptr_to_str<'a>(p: *const c_char) -> &'a str {
    if p.is_null() { "" } else { CStr::from_ptr(p).to_str().unwrap_or("") }
}

fn to_cstring(s: &str) -> *mut c_char {
    CString::new(s).map(|s| s.into_raw()).unwrap_or(ptr::null_mut())
}

#[repr(C)]
pub struct OwClient {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct OwStatus {
    pub ok: bool,
    pub error_message: *mut c_char,
}

impl OwStatus {
    fn success() -> Self { Self { ok: true, error_message: ptr::null_mut() } }
    fn error(msg: &str) -> Self { Self { ok: false, error_message: to_cstring(msg) } }
}

#[repr(C)]
pub struct OwSandbox {
    pub sandbox_id: *mut c_char,
    pub name: *mut c_char,
    pub status: *mut c_char,
}

struct ClientInner(overlayward_sdk::Client);

#[no_mangle]
pub extern "C" fn ow_client_new(endpoint: *const c_char, token: *const c_char) -> *mut OwClient {
    let endpoint = unsafe { ptr_to_str(endpoint) };
    let token = unsafe { ptr_to_str(token) };
    let ep = if endpoint.is_empty() { "http://localhost:8421" } else { endpoint };
    let cfg = overlayward_sdk::Config { endpoint: ep.into(), token: token.into() };
    match rt().block_on(overlayward_sdk::Client::new(cfg)) {
        Ok(client) => Box::into_raw(Box::new(ClientInner(client))) as *mut OwClient,
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn ow_client_free(client: *mut OwClient) {
    if !client.is_null() { unsafe { drop(Box::from_raw(client as *mut ClientInner)); } }
}

#[inline(always)]
fn get_client(client: *mut OwClient) -> Option<&'static ClientInner> {
    if client.is_null() { None } else { Some(unsafe { &*(client as *const ClientInner) }) }
}

#[no_mangle]
pub extern "C" fn ow_sandbox_create(client: *mut OwClient, name: *const c_char, cpu: i32, memory: *const c_char) -> *mut OwSandbox {
    let Some(c) = get_client(client) else { return ptr::null_mut() };
    let name_s = unsafe { ptr_to_str(name) };
    let mem_s = unsafe { ptr_to_str(memory) };
    let mem_s = if mem_s.is_empty() { "4GB" } else { mem_s };
    let req = ow_gateway::proto::CreateSandboxRequest {
        name: name_s.into(), cpu, memory: mem_s.into(),
        disk: "20GB".into(), image: "ubuntu:24.04".into(),
        gpu: None, labels: Default::default(), network_policy: None,
    };
    match rt().block_on(c.0.sandbox().create(req)) {
        Ok(sb) => Box::into_raw(Box::new(OwSandbox {
            sandbox_id: to_cstring(&sb.sandbox_id),
            name: to_cstring(&sb.name),
            status: to_cstring(&sb.status),
        })),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn ow_sandbox_start(client: *mut OwClient, sandbox_id: *const c_char) -> OwStatus {
    let Some(c) = get_client(client) else { return OwStatus::error("null client") };
    let id = unsafe { ptr_to_str(sandbox_id) };
    match rt().block_on(c.0.sandbox().start(id)) {
        Ok(()) => OwStatus::success(),
        Err(e) => OwStatus::error(&e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn ow_sandbox_stop(client: *mut OwClient, sandbox_id: *const c_char, force: bool) -> OwStatus {
    let Some(c) = get_client(client) else { return OwStatus::error("null client") };
    let id = unsafe { ptr_to_str(sandbox_id) };
    match rt().block_on(c.0.sandbox().stop(id, force)) {
        Ok(()) => OwStatus::success(),
        Err(e) => OwStatus::error(&e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn ow_sandbox_destroy(client: *mut OwClient, sandbox_id: *const c_char) -> OwStatus {
    let Some(c) = get_client(client) else { return OwStatus::error("null client") };
    let id = unsafe { ptr_to_str(sandbox_id) };
    match rt().block_on(c.0.sandbox().destroy(id, false, true)) {
        Ok(()) => OwStatus::success(),
        Err(e) => OwStatus::error(&e.to_string()),
    }
}

#[no_mangle]
pub extern "C" fn ow_sandbox_free(sb: *mut OwSandbox) {
    if !sb.is_null() {
        unsafe {
            let s = Box::from_raw(sb);
            if !s.sandbox_id.is_null() { drop(CString::from_raw(s.sandbox_id)); }
            if !s.name.is_null() { drop(CString::from_raw(s.name)); }
            if !s.status.is_null() { drop(CString::from_raw(s.status)); }
        }
    }
}

#[no_mangle]
pub extern "C" fn ow_status_free_message(msg: *mut c_char) {
    if !msg.is_null() { unsafe { drop(CString::from_raw(msg)); } }
}
