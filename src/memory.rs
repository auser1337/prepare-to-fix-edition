use windows::Win32::System::Memory::{PAGE_PROTECTION_FLAGS, PAGE_READWRITE, VirtualProtect};

pub(crate) unsafe fn protect_rw(address: usize, size: usize) -> windows::core::Result<()> {
    let mut old_protect: PAGE_PROTECTION_FLAGS = Default::default();
    unsafe { VirtualProtect(address as *const _, size, PAGE_READWRITE, &mut old_protect) }
}
