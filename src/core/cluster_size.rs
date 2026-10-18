use std::path::Path;

use super::path_guard;

pub const DEFAULT_CLUSTER_SIZE: i32 = 4096;

pub fn get_cluster_size(path: Option<&str>) -> i32 {
    let Some(normalized) = path_guard::try_normalize(path) else {
        return DEFAULT_CLUSTER_SIZE;
    };
    if !Path::new(&normalized).is_dir() {
        return DEFAULT_CLUSTER_SIZE;
    }

    let root = match normalized.find(':') {
        Some(index) => normalized[..index + 1].to_string(),
        None => return DEFAULT_CLUSTER_SIZE,
    };

    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceW;
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut sectors_per_cluster: u32 = 0;
    let mut bytes_per_sector: u32 = 0;
    let mut free_clusters: u32 = 0;
    let mut total_clusters: u32 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceW(
            wide.as_ptr(),
            &mut sectors_per_cluster,
            &mut bytes_per_sector,
            &mut free_clusters,
            &mut total_clusters,
        )
    };
    if ok == 0 || sectors_per_cluster == 0 || bytes_per_sector == 0 {
        return DEFAULT_CLUSTER_SIZE;
    }

    let cluster_size = sectors_per_cluster as u64 * bytes_per_sector as u64;
    if cluster_size > 0 && cluster_size <= 1024 * 1024 {
        cluster_size as i32
    } else {
        DEFAULT_CLUSTER_SIZE
    }
}
