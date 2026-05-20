use anyhow::Result;

#[cfg(target_os = "macos")]
mod platform {
  use std::ffi::{c_void, CStr, CString};
  use std::ptr;

  use anyhow::{bail, Context, Result};

  type OSStatus = i32;
  type SecKeychainItemRef = *mut c_void;

  const ERR_SEC_DUPLICATE_ITEM: OSStatus = -25299;
  const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;
  const SERVICE_NAME: &str = "app.pith.plugin-connectors";

  #[link(name = "Security", kind = "framework")]
  unsafe extern "C" {
    fn SecKeychainAddGenericPassword(
      keychain: *mut c_void,
      service_name_length: u32,
      service_name: *const i8,
      account_name_length: u32,
      account_name: *const i8,
      password_length: u32,
      password_data: *const c_void,
      item_ref: *mut SecKeychainItemRef,
    ) -> OSStatus;
    fn SecKeychainFindGenericPassword(
      keychain_or_array: *mut c_void,
      service_name_length: u32,
      service_name: *const i8,
      account_name_length: u32,
      account_name: *const i8,
      password_length: *mut u32,
      password_data: *mut *mut c_void,
      item_ref: *mut SecKeychainItemRef,
    ) -> OSStatus;
    fn SecKeychainItemModifyAttributesAndData(
      item_ref: SecKeychainItemRef,
      attr_list: *const c_void,
      length: u32,
      data: *const c_void,
    ) -> OSStatus;
    fn SecKeychainItemDelete(item_ref: SecKeychainItemRef) -> OSStatus;
    fn SecKeychainItemFreeContent(attr_list: *mut c_void, data: *mut c_void) -> OSStatus;
  }

  #[link(name = "CoreFoundation", kind = "framework")]
  unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
  }

  pub(super) fn save_secret(connector_id: &str, secret: &str) -> Result<()> {
    let service = c_string(SERVICE_NAME, "Keychain service")?;
    let account = c_string(connector_id, "connector credential account")?;
    let secret_bytes = secret.as_bytes();
    let mut item_ref: SecKeychainItemRef = ptr::null_mut();

    let status = unsafe {
      SecKeychainAddGenericPassword(
        ptr::null_mut(),
        c_len(&service),
        service.as_ptr(),
        c_len(&account),
        account.as_ptr(),
        secret_bytes.len() as u32,
        secret_bytes.as_ptr().cast(),
        &mut item_ref,
      )
    };
    if !item_ref.is_null() {
      unsafe {
        CFRelease(item_ref.cast());
      }
    }

    if status == ERR_SEC_DUPLICATE_ITEM {
      return update_existing_secret(connector_id, secret);
    }
    ensure_success(status, "save connector credential in Keychain")
  }

  pub(super) fn load_secret(connector_id: &str) -> Option<String> {
    find_secret(connector_id).ok().flatten()
  }

  pub(super) fn delete_secret(connector_id: &str) -> Result<()> {
    let Some(item_ref) = find_item(connector_id)? else {
      return Ok(());
    };
    let status = unsafe { SecKeychainItemDelete(item_ref) };
    unsafe {
      CFRelease(item_ref.cast());
    }
    ensure_success(status, "delete connector credential from Keychain")
  }

  fn update_existing_secret(connector_id: &str, secret: &str) -> Result<()> {
    let Some(item_ref) = find_item(connector_id)? else {
      bail!("Keychain item disappeared before credential update");
    };
    let secret_bytes = secret.as_bytes();
    let status = unsafe {
      SecKeychainItemModifyAttributesAndData(
        item_ref,
        ptr::null(),
        secret_bytes.len() as u32,
        secret_bytes.as_ptr().cast(),
      )
    };
    unsafe {
      CFRelease(item_ref.cast());
    }
    ensure_success(status, "update connector credential in Keychain")
  }

  fn find_secret(connector_id: &str) -> Result<Option<String>> {
    let service = c_string(SERVICE_NAME, "Keychain service")?;
    let account = c_string(connector_id, "connector credential account")?;
    let mut password_length = 0_u32;
    let mut password_data: *mut c_void = ptr::null_mut();
    let mut item_ref: SecKeychainItemRef = ptr::null_mut();
    let status = unsafe {
      SecKeychainFindGenericPassword(
        ptr::null_mut(),
        c_len(&service),
        service.as_ptr(),
        c_len(&account),
        account.as_ptr(),
        &mut password_length,
        &mut password_data,
        &mut item_ref,
      )
    };

    if status == ERR_SEC_ITEM_NOT_FOUND {
      return Ok(None);
    }
    ensure_success(status, "load connector credential from Keychain")?;

    let secret = if password_data.is_null() {
      None
    } else {
      let bytes =
        unsafe { std::slice::from_raw_parts(password_data.cast::<u8>(), password_length as usize) };
      String::from_utf8(bytes.to_vec()).ok()
    };

    unsafe {
      SecKeychainItemFreeContent(ptr::null_mut(), password_data);
      if !item_ref.is_null() {
        CFRelease(item_ref.cast());
      }
    }

    Ok(secret)
  }

  fn find_item(connector_id: &str) -> Result<Option<SecKeychainItemRef>> {
    let service = c_string(SERVICE_NAME, "Keychain service")?;
    let account = c_string(connector_id, "connector credential account")?;
    let mut item_ref: SecKeychainItemRef = ptr::null_mut();
    let status = unsafe {
      SecKeychainFindGenericPassword(
        ptr::null_mut(),
        c_len(&service),
        service.as_ptr(),
        c_len(&account),
        account.as_ptr(),
        ptr::null_mut(),
        ptr::null_mut(),
        &mut item_ref,
      )
    };
    if status == ERR_SEC_ITEM_NOT_FOUND {
      return Ok(None);
    }
    ensure_success(status, "find connector credential in Keychain")?;
    Ok(Some(item_ref))
  }

  fn c_string(value: &str, label: &str) -> Result<CString> {
    CString::new(value).with_context(|| format!("{label} contains an unsupported NUL byte"))
  }

  fn c_len(value: &CStr) -> u32 {
    value.to_bytes().len() as u32
  }

  fn ensure_success(status: OSStatus, operation: &str) -> Result<()> {
    if status == 0 {
      return Ok(());
    }
    bail!("{operation} failed with OSStatus {status}")
  }
}

#[cfg(not(target_os = "macos"))]
mod platform {
  use anyhow::Result;

  pub(super) fn save_secret(_connector_id: &str, _secret: &str) -> Result<()> {
    Ok(())
  }

  pub(super) fn load_secret(_connector_id: &str) -> Option<String> {
    None
  }

  pub(super) fn delete_secret(_connector_id: &str) -> Result<()> {
    Ok(())
  }
}

pub(crate) fn save_connector_secret(connector_id: &str, secret: &str) -> Result<()> {
  platform::save_secret(connector_id, secret)
}

pub(crate) fn load_connector_secret(connector_id: &str) -> Option<String> {
  platform::load_secret(connector_id)
}

pub(crate) fn delete_connector_secret(connector_id: &str) -> Result<()> {
  platform::delete_secret(connector_id)
}
