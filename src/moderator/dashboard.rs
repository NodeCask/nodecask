use crate::common::AppState;
use crate::moderator::{Data, ModContext as Context};
use crate::store::Store;
use axum::extract::State;
use axum::response::IntoResponse;
use sysinfo::{Disks, System};

#[axum::debug_handler(state = AppState)]
pub async fn dashboard(
    _ctx: Context,
    _store: Store,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let topic_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM topic")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let comment_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comment")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let sqlite_version: String = sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(&state.pool)
        .await
        .unwrap_or("Unknown".to_string());

    // System Info
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let os_info = format!("{} {} (Kernel: {})", os_name, os_version, kernel_version);

    // Disk Info
    let disks = Disks::new_with_refreshed_list();
    let mut disk_list = Vec::new();
    for disk in &disks {
        let fs_type = disk.file_system().to_str().unwrap_or_default();
        // Filter out common virtual file systems
        // Adjust list as needed.
        if fs_type == "tmpfs"
            || fs_type == "overlay"
            || fs_type == "sysfs"
            || fs_type == "proc"
            || fs_type == "devtmpfs"
            || fs_type == "squashfs"
        {
            continue;
        }
        if Some("/boot/efi") == disk.mount_point().to_str() {
            continue;
        }

        disk_list.push(serde_json::json!({
            "name": disk.name().to_str().unwrap_or_default(),
            "mount_point": disk.mount_point().to_str().unwrap_or_default(),
            "total_space": disk.total_space(),
            "available_space": disk.available_space(),
            "file_system": fs_type
        }));
    }

    let mut sys = System::new_all();
    sys.refresh_all();

    Data::ok(serde_json::json!({
        "hostname": hostname,
        "os_info": os_info,
        "content_stats": {
            "user_count": user_count,
            "topic_count": topic_count,
            "comment_count": comment_count
        },
        "disks": disk_list,
        "sqlite_version": sqlite_version,
        "build_os": crate::build::BUILD_OS,
        "build_time": crate::build::BUILD_TIME,
        "rust_version": crate::build::RUST_VERSION,
        "available_memory": sys.available_memory(),
        "total_memory": sys.total_memory(),
        "used_memory": sys.used_memory(),
        "boot_time": System::boot_time(),
    }))
    .into_response()
}

pub async fn me(
    ctx: Context,
    _store: Store,
) -> impl IntoResponse {
    Data::ok(ctx.moderator).into_response()
}

pub async fn logout(
    ctx: Context,
    store: Store,
) -> impl IntoResponse {
    let _ = store.delete_access_token(&ctx.token).await;
    Data::done().into_response()
}

