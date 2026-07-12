//! MLX allocator memory management.

use super::ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryStats {
    pub active: usize,
    pub cache: usize,
    pub peak: usize,
}

pub fn clear_cache() -> Result<(), i32> {
    let status = unsafe { ffi::mlx_clear_cache() };
    if status == 0 {
        Ok(())
    } else {
        Err(status)
    }
}

pub fn set_cache_limit(limit: usize) -> Result<usize, i32> {
    let mut previous = 0;
    let status = unsafe { ffi::mlx_set_cache_limit(&mut previous, limit) };
    if status == 0 {
        Ok(previous)
    } else {
        Err(status)
    }
}

pub fn reset_peak() -> Result<(), i32> {
    let status = unsafe { ffi::mlx_reset_peak_memory() };
    if status == 0 {
        Ok(())
    } else {
        Err(status)
    }
}

pub fn stats() -> Result<MemoryStats, i32> {
    let mut active = 0;
    let mut cache = 0;
    let mut peak = 0;

    let active_status = unsafe { ffi::mlx_get_active_memory(&mut active) };
    let cache_status = unsafe { ffi::mlx_get_cache_memory(&mut cache) };
    let peak_status = unsafe { ffi::mlx_get_peak_memory(&mut peak) };
    for status in [active_status, cache_status, peak_status] {
        if status != 0 {
            return Err(status);
        }
    }

    Ok(MemoryStats {
        active,
        cache,
        peak,
    })
}
